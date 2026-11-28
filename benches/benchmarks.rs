//! Criterion benchmarks of the hot paths: the virtual DOM diff, style resolution, layout,
//! painting, script calls and the whole render pipeline.

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion, criterion_group, criterion_main};
use std::fmt::Write;
use std::hint::black_box;

use cranky::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use cranky::features::layout_engine::domain::{
    AlignItems, BoxMargin, FlexDirection, JustifyContent, LayoutRoot, NodePath, StyledNode,
    SurfaceSpace,
};
use cranky::features::layout_engine::ports::LayoutEnginePort;
use cranky::features::module_runtime::domain::render_pipeline::{
    DiffInput, LayoutContext, RenderPipeline,
};
use cranky::features::module_runtime::ports::{AnyModulePort, ModuleInitError};
use cranky::features::styling::adapters::lightningcss::LightningCssAdapter;
use cranky::features::styling::domain::{
    ClassName, ClassNameList, ComputedStyle, ElementIdentity, ElementQuery, InheritedStyle,
    PseudoClass, StyleSheetName,
};
use cranky::features::styling::ports::{CssParserPort, ParsedStyleSheetPort, StyleResolverPort};
use cranky::features::vdom::adapters::DefaultVdomDiffAdapter;
use cranky::features::vdom::domain::{ElementSelectors, Interactions, NodeKey, TextContent, VNode};
use cranky::features::vdom::ports::VdomDiffPort;
use cranky::shared::config::domain::{AxisMargins, Config, FontFamily, FontSize, ModuleConfig};
use cranky::shared::dbus::domain::DBusSubscription;
use cranky::shared::events::signals::{SignalHub, SignalKind};
use cranky::shared::primitives::color::{Color, DrawingColor};
use cranky::shared::primitives::geometry::{LogicalPx, Position, Rect, Scale, Size};
use cranky::shared::primitives::{
    FunctionName, ModuleOptions, MonitorId, SizeConstraint, TextAlignment,
};
use cranky::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory;
use cranky::shared::rendering::ports::canvas::{Canvas, CanvasFactory};
use cranky::shared::rendering::ports::draw::{BorderStroke, PaintTarget, RoundedRect};
use cranky::shared::rendering::ports::font::Font;
use cranky::shared::rendering::ports::text::{TextFont, TextPlacement, TextRun};
use cranky::shared::scripting::adapters::lua::LuaModule;
use cranky::shared::scripting::adapters::rhai::RhaiModule;

// ----------------------------------------------------------------------------
// Helpers for Benchmark Fixtures
// ----------------------------------------------------------------------------

fn create_test_vnode_tree(depth: usize, branching: usize, prefix: &str) -> VNode {
    if depth == 0 {
        let node = VNode::new_text(
            TextContent::new(format!("Text {prefix}")),
            class_selectors("item-label"),
            Interactions::default(),
        );
        with_key_if_valid(node, format!("{prefix}_leaf"))
    } else {
        let children = (0..branching)
            .map(|index| {
                let child_prefix = format!("{prefix}_{depth}_{index}");
                create_test_vnode_tree(depth.saturating_sub(1), branching, &child_prefix)
            })
            .collect();
        let node = VNode::new_flex(
            children,
            class_selectors("container-box"),
            Interactions::default(),
        );
        with_key_if_valid(node, format!("{prefix}_flex"))
    }
}

/// Selectors carrying the one class `class`, or none when it is not a valid class name.
fn class_selectors(class: &str) -> ElementSelectors {
    let classes = ClassName::new(class)
        .ok()
        .map(|class| ClassNameList::new(vec![class]));
    ElementSelectors::new(classes, None)
}

/// `node` keyed by `key`, or left unkeyed when `key` is not a valid node key.
fn with_key_if_valid(node: VNode, key: String) -> VNode {
    match NodeKey::new(key) {
        Ok(key) => node.with_key(key),
        Err(_) => node,
    }
}

fn create_styled_node_tree(depth: usize, branching: usize) -> StyledNode {
    let style = benchmark_node_style();
    if depth == 0 {
        styled_text_leaf(style)
    } else {
        let children = (0..branching)
            .map(|_| create_styled_node_tree(depth.saturating_sub(1), branching))
            .collect();
        StyledNode::Flex {
            path: NodePath::root_in(SurfaceSpace::Bar),
            node_key: None,
            children,
            style,
            on_click: None,
            on_hover: None,
            tooltip: None,
            popup: None,
            panel: None,
        }
    }
}

/// The text node at the bottom of a styled benchmark tree.
fn styled_text_leaf(style: ComputedStyle) -> StyledNode {
    StyledNode::Text {
        path: NodePath::root_in(SurfaceSpace::Bar),
        node_key: None,
        text: TextContent::new("Benchmark Text Content".to_string()),
        style,
        on_click: None,
        on_hover: None,
        tooltip: None,
        popup: None,
        panel: None,
    }
}

/// The style every node of a styled benchmark tree carries: a centred,
/// spaced-out row with a margin of one on every side.
fn benchmark_node_style() -> ComputedStyle {
    let mut style = ComputedStyle::default();
    style.set_flex_direction(FlexDirection::Row);
    style.set_justify_content(JustifyContent::SpaceBetween);
    style.set_align_items(AlignItems::Center);
    style.set_margin(BoxMargin::new(
        AxisMargins::new(1.0, 1.0),
        AxisMargins::new(1.0, 1.0),
    ));
    style
}

struct StaticStyleResolver {
    style: ComputedStyle,
}

impl StyleResolverPort for StaticStyleResolver {
    fn resolve_style(
        &self,
        _query: &ElementQuery<'_>,
        _inherited: &InheritedStyle,
    ) -> (ComputedStyle, InheritedStyle) {
        (self.style.clone(), InheritedStyle::default())
    }
}

struct BenchMockModule {
    vnode: VNode,
}

impl AnyModulePort for BenchMockModule {
    fn init(
        &mut self,
        _config: &ModuleConfig,
        _full_config: &Config,
    ) -> Result<(), ModuleInitError> {
        Ok(())
    }

    fn subscriptions(&self) -> &[SignalKind] {
        &[]
    }

    fn dbus_subscriptions(&self) -> &[DBusSubscription] {
        &[]
    }

    fn styles(&self) -> &[StyleSheetName] {
        &[]
    }

    fn refresh(&mut self, _hub: &SignalHub, _changed_signals: &[SignalKind]) {}

    fn render(&self, _monitor: &MonitorId) -> VNode {
        self.vnode.clone()
    }

    fn call_function_with_args(
        &mut self,
        _name: &FunctionName,
        _args: &[&str],
    ) -> Result<(), ModuleInitError> {
        Ok(())
    }
}

// ----------------------------------------------------------------------------
// 1. VDOM Benchmarks
// ----------------------------------------------------------------------------

/// One diff benchmark: the name criterion reports it under, the previous
/// render and the new one.
struct DiffCase<'a> {
    name: &'a str,
    previous: &'a VNode,
    next: &'a VNode,
}

impl<'a> DiffCase<'a> {
    const fn new(name: &'a str, previous: &'a VNode, next: &'a VNode) -> Self {
        Self {
            name,
            previous,
            next,
        }
    }
}

/// One benchmark over a single input: the name criterion reports it under and
/// the input it times.
struct BenchCase<'a, Input> {
    name: &'a str,
    input: &'a Input,
}

impl<'a, Input> BenchCase<'a, Input> {
    const fn new(name: &'a str, input: &'a Input) -> Self {
        Self { name, input }
    }
}

fn bench_vdom(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("vdom");

    // Small tree: depth 1, branching 4 (~5 nodes)
    let small = tree_pair(1, 4, "small");

    // Medium tree: depth 3, branching 3 (~40 nodes)
    let medium = tree_pair(3, 3, "med");

    // Large tree: depth 4, branching 3 (~120 nodes)
    let large = tree_pair(4, 3, "large");

    let cases = [
        DiffCase::new("diff_small_identical", &small.0, &small.0),
        DiffCase::new("diff_small_mutated", &small.0, &small.1),
        DiffCase::new("diff_medium_mutated", &medium.0, &medium.1),
        DiffCase::new("diff_large_mutated", &large.0, &large.1),
    ];
    for case in &cases {
        bench_diff(&mut group, case);
    }

    group.finish();
}

/// A tree of the given shape keyed by `prefix`, and the same shape keyed by
/// `prefix` and `_mutated`, so every key differs between the two.
fn tree_pair(depth: usize, branching: usize, prefix: &str) -> (VNode, VNode) {
    (
        create_test_vnode_tree(depth, branching, prefix),
        create_test_vnode_tree(depth, branching, &format!("{prefix}_mutated")),
    )
}

/// Times one diff of `case` with the default differ, built outside the timed loop.
fn bench_diff(group: &mut BenchmarkGroup<'_, WallTime>, case: &DiffCase<'_>) {
    let differ = DefaultVdomDiffAdapter::new();
    let DiffCase {
        name,
        previous,
        next,
    } = *case;
    group.bench_function(name, |bencher| {
        bencher.iter(|| {
            let res = differ.diff(black_box(Some(previous)), black_box(next));
            black_box(res);
        });
    });
}

// ----------------------------------------------------------------------------
// 2. Styling Benchmarks
// ----------------------------------------------------------------------------

const SAMPLE_CSS: &str = r"
        .container-box {
            display: flex;
            flex-direction: row;
            justify-content: space-between;
            align-items: center;
            background-color: #1e1e2e;
            color: #cdd6f4;
            padding: 4px 8px;
            margin: 2px;
            border-radius: 6px;
            border: 1px solid #313244;
            font-size: 14px;
        }

        .container-box:hover {
            background-color: #313244;
            color: #89b4fa;
        }

        .item-label {
            color: #a6adc8;
            font-size: 12px;
            margin-right: 4px;
        }

        #active-workspace {
            background-color: #89b4fa;
            color: #11111b;
            font-weight: bold;
        }

        .urgent {
            background-color: #f38ba8;
            color: #11111b;
        }
    ";

fn bench_styling(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("styling");
    let parser = LightningCssAdapter::new();

    let Ok(sheet_name) = StyleSheetName::new("sample") else {
        return;
    };

    group.bench_function("parse_css_stylesheet", |bencher| {
        bencher.iter(|| {
            let parsed =
                parser.parse_stylesheet(black_box(sheet_name.clone()), black_box(SAMPLE_CSS));
            let _ = black_box(parsed);
        });
    });

    let Ok(parsed_sheet) = parser.parse_stylesheet(sheet_name, SAMPLE_CSS) else {
        return;
    };
    bench_sample_queries(&mut group, &*parsed_sheet);

    group.finish();
}

/// Times resolving the sample sheet for a plain label and for a hovered container.
fn bench_sample_queries(
    group: &mut BenchmarkGroup<'_, WallTime>,
    sheet: &dyn ParsedStyleSheetPort,
) {
    let (Ok(class_container), Ok(class_label)) = (
        ClassName::new("container-box"),
        ClassName::new("item-label"),
    ) else {
        return;
    };
    let classes_hover = [class_container];
    let pseudos_hover = [PseudoClass::Hover];
    let query_hover = ElementQuery::new(
        ElementIdentity::new("flex", None, &classes_hover),
        &pseudos_hover,
        None,
    );

    let classes_simple = [class_label];
    let query_simple = ElementQuery::new(
        ElementIdentity::new("text", None, &classes_simple),
        &[],
        None,
    );

    let simple = BenchCase::new("resolve_style_simple_match", &query_simple);
    let hover = BenchCase::new("resolve_style_pseudo_class_match", &query_hover);
    bench_resolve(group, sheet, &simple);
    bench_resolve(group, sheet, &hover);
}

/// Times resolving `sheet` for the query of `case` with nothing inherited.
fn bench_resolve(
    group: &mut BenchmarkGroup<'_, WallTime>,
    sheet: &dyn ParsedStyleSheetPort,
    case: &BenchCase<'_, ElementQuery<'_>>,
) {
    let BenchCase { name, input: query } = *case;
    group.bench_function(name, |bencher| {
        bencher.iter(|| {
            let style =
                sheet.resolve_style(black_box(query), black_box(&InheritedStyle::default()));
            black_box(style);
        });
    });
}

/// A sheet with many distinct-class rules and a query matching exactly one
/// of them — the shape the rule-bucket index (`RuleIndex`) exists for.
/// `bench_styling`'s benchmarks above have too few rules (6) to show a
/// difference between an indexed lookup and a full linear scan; this one is
/// sized so the win is actually visible in the numbers.
fn bench_styling_rule_buckets(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("styling");
    let parser = LightningCssAdapter::new();

    let Ok(many_rules_name) = StyleSheetName::new("many_rules") else {
        return;
    };
    let Ok(many_rules_sheet) = parser.parse_stylesheet(many_rules_name, &many_rules_css()) else {
        return;
    };

    let Ok(class_hot) = ClassName::new("hot-class") else {
        return;
    };
    let classes_hot = [class_hot];
    let query_many_rules =
        ElementQuery::new(ElementIdentity::new("text", None, &classes_hot), &[], None);

    let case = BenchCase::new("resolve_style_many_rules_one_match", &query_many_rules);
    bench_resolve(&mut group, &*many_rules_sheet, &case);

    group.finish();
}

/// Eighty rules of one class each, then the `.hot-class` rule the query matches.
fn many_rules_css() -> String {
    let mut many_rules_css = String::new();
    for index in 0..80 {
        let _ = writeln!(
            many_rules_css,
            ".class-{index} {{ color: #ffffff; background-color: #1a1b26; padding: 4px; }}"
        );
    }
    many_rules_css
        .push_str(".hot-class { color: #7aa2f7; background-color: #24283b; font-size: 14px; }\n");
    many_rules_css
}

// ----------------------------------------------------------------------------
// 3. Layout Engine Benchmarks
// ----------------------------------------------------------------------------

fn bench_layout(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("layout");
    let mut canvas_factory = TinySkiaCanvasFactory::new();

    let tree_small = create_styled_node_tree(1, 4);
    let tree_medium = create_styled_node_tree(3, 3);
    let tree_large = create_styled_node_tree(4, 3);

    let mut bench_tree = |name: &str, tree: &StyledNode| {
        bench_taffy_layout(&mut group, &mut canvas_factory, &BenchCase::new(name, tree));
    };
    bench_tree("taffy_layout_small_tree", &tree_small);
    bench_tree("taffy_layout_medium_tree", &tree_medium);
    bench_tree("taffy_layout_large_tree", &tree_large);

    group.finish();
}

/// Times laying out a clone of the tree of `case` with a fresh text measurer
/// each iteration, the layout engine built once outside the timed loop.
fn bench_taffy_layout(
    group: &mut BenchmarkGroup<'_, WallTime>,
    canvas_factory: &mut TinySkiaCanvasFactory,
    case: &BenchCase<'_, StyledNode>,
) {
    let BenchCase { name, input: tree } = *case;
    group.bench_function(name, |bencher| {
        let mut layout_engine = TaffyLayoutAdapter::default();
        bencher.iter(|| {
            let mut measurer = canvas_factory.create_text_measurer(
                Scale::new(1.0),
                Font::new(FontFamily::new(String::new()), FontSize::new(14.0)),
            );
            let res = layout_engine.calculate_layout(
                LayoutRoot::new(
                    black_box(tree.clone()),
                    Position::new(0, 0),
                    SizeConstraint::none(),
                ),
                &mut measurer,
            );
            let _ = black_box(res);
        });
    });
}

// ----------------------------------------------------------------------------
// 4. Rendering Subsystem Benchmarks
// ----------------------------------------------------------------------------

/// What every rendering benchmark paints into: an 800x40 buffer and the
/// factory that makes a canvas over it.
struct PaintFixture {
    canvas_factory: TinySkiaCanvasFactory,
    size: Size,
    buffer: Vec<u8>,
}

impl PaintFixture {
    fn new() -> Self {
        Self {
            canvas_factory: TinySkiaCanvasFactory::new(),
            size: Size::new(800, 40),
            buffer: vec![0_u8; 800_usize.saturating_mul(40).saturating_mul(4)],
        }
    }

    /// A canvas over the whole buffer at scale 1, made inside the timed loop
    /// as each benchmark did.
    fn canvas(&mut self) -> impl Canvas + '_ {
        self.canvas_factory.create_canvas(
            PaintTarget::new(
                black_box(&mut self.buffer),
                black_box(self.size),
                Scale::new(1.0),
            ),
            Font::new(FontFamily::new(String::new()), FontSize::new(14.0)),
        )
    }
}

fn bench_rendering(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("rendering");
    let mut fixture = PaintFixture::new();
    bench_draw_rect_and_border(&mut group, &mut fixture);
    bench_draw_text(&mut group, &mut fixture);
    group.finish();
}

fn bench_draw_rect_and_border(
    group: &mut BenchmarkGroup<'_, WallTime>,
    fixture: &mut PaintFixture,
) {
    let bg_color = DrawingColor::Solid(Color::rgb(30, 30, 46));
    let border_color = DrawingColor::Solid(Color::rgb(137, 180, 250));
    let size = fixture.size;

    group.bench_function("tinyskia_draw_rect_and_border", |bencher| {
        bencher.iter(|| {
            let mut canvas = fixture.canvas();

            let shape = RoundedRect::new(Rect::new(Position::new(0, 0), size), LogicalPx::new(4.0));
            canvas.draw_rect(shape, bg_color.clone());

            canvas.draw_border(
                shape,
                BorderStroke::new(border_color.clone(), LogicalPx::new(1.0)),
            );
        });
    });
}

fn bench_draw_text(group: &mut BenchmarkGroup<'_, WallTime>, fixture: &mut PaintFixture) {
    let text_color = DrawingColor::Solid(Color::rgb(205, 214, 244));

    group.bench_function("tinyskia_draw_text", |bencher| {
        bencher.iter(|| {
            let mut canvas = fixture.canvas();

            canvas.draw_text(
                TextRun::new(
                    black_box("Workspace 1: Web | Cranky Bar Benchmark Testing"),
                    TextFont::new(None, Some(FontSize::new(13.0))),
                    text_color.clone(),
                ),
                TextPlacement::new(Position::new(12, 10), TextAlignment::default()),
            );
        });
    });
}

// ----------------------------------------------------------------------------
// 5. Scripting Subsystem Benchmarks
// ----------------------------------------------------------------------------

const LUA_SOURCE: &str = r#"
        function metadata()
            return { subscriptions = {"time"} }
        end

        function render(ctx)
            return {
                tag = "flex",
                class = "container",
                children = {
                    { tag = "text", text = "Clock: 12:34:56", class = "time-label" },
                    { tag = "text", text = "CPU: 42%", class = "metrics-label" }
                }
            }
        end
    "#;

const RHAI_SOURCE: &str = r#"
        fn metadata() {
            #{ subscriptions: ["time"] }
        }

        fn render(ctx) {
            #{
                tag: "flex",
                class: "container",
                children: [
                    #{ tag: "text", text: "Clock: 12:34:56", class: "time-label" },
                    #{ tag: "text", text: "CPU: 42%", class: "metrics-label" }
                ]
            }
        }
    "#;

fn bench_scripting(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("scripting");
    let monitor_id = MonitorId::new("DP-1");

    let config = Config::default();
    let module_config = ModuleConfig::new("test_bench".into(), ModuleOptions::default());

    let mut lua_mod = LuaModule::new("test_lua".to_string(), LUA_SOURCE.to_string());
    let _ = lua_mod.init(&module_config, &config);

    let Ok(mut rhai_mod) = RhaiModule::new("test_rhai".to_string(), RHAI_SOURCE) else {
        return;
    };
    let _ = rhai_mod.init(&module_config, &config);

    group.bench_function("lua_module_render", |bencher| {
        bencher.iter(|| {
            let node = lua_mod.render(black_box(&monitor_id));
            black_box(node);
        });
    });

    group.bench_function("rhai_module_render", |bencher| {
        bencher.iter(|| {
            let node = rhai_mod.render(black_box(&monitor_id));
            black_box(node);
        });
    });

    group.finish();
}

// ----------------------------------------------------------------------------
// 6. Full Render Pipeline Benchmark
// ----------------------------------------------------------------------------

/// What one pipeline iteration reads and never changes: the module, its
/// monitor, the differ and a resolver that gives every node the same style.
struct PipelineFixture {
    monitor_id: MonitorId,
    module: BenchMockModule,
    vdom_diff: DefaultVdomDiffAdapter,
    style_resolver: StaticStyleResolver,
}

impl PipelineFixture {
    fn new() -> Self {
        let mut computed_style = ComputedStyle::default();
        computed_style.set_flex_direction(FlexDirection::Row);
        computed_style.set_justify_content(JustifyContent::SpaceBetween);
        computed_style.set_align_items(AlignItems::Center);
        Self {
            monitor_id: MonitorId::new("DP-1"),
            module: BenchMockModule {
                vnode: create_test_vnode_tree(2, 3, "pipe"),
            },
            vdom_diff: DefaultVdomDiffAdapter::new(),
            style_resolver: StaticStyleResolver {
                style: computed_style,
            },
        }
    }

    /// The diff inputs of a module with no bounds, child sizes or pointer yet.
    fn diff_input(&self) -> DiffInput<'_> {
        DiffInput {
            monitor_id: &self.monitor_id,
            vdom_diff: &self.vdom_diff,
            bounds: None,
            child_sizes: None,
            interaction: None,
        }
    }
}

/// The four layout engines a pipeline iteration lays its surfaces out with.
#[derive(Default)]
struct LayoutEngines {
    layout: TaffyLayoutAdapter,
    popup: TaffyLayoutAdapter,
    panel: TaffyLayoutAdapter,
    tooltip: TaffyLayoutAdapter,
}

/// The layout inputs of the fixture's module at scale 1, with no bounds yet.
///
/// A macro rather than a function because the map of child errors has a type
/// the benchmarks cannot name, so only an inferred local can hold it.
macro_rules! layout_context {
    ($fixture:expr, $child_errors:expr, $canvas_factory:expr, $engines:expr) => {
        LayoutContext {
            monitor_id: &$fixture.monitor_id,
            scale: Scale::new(1.0),
            style_resolver: &$fixture.style_resolver,
            child_errors: $child_errors,
            current_bounds: None,
            current_constraint: cranky::shared::primitives::SizeConstraint::none(),
            current_child_sizes: None,
            interaction_context: None,
            canvas_factory: $canvas_factory,
            layout_engine: &mut $engines.layout,
            popup_layout_engine: &mut $engines.popup,
            panel_layout_engine: &mut $engines.panel,
            tooltip_layout_engine: &mut $engines.tooltip,
        }
    };
}

/// Runs one full render pipeline iteration of the fixture's module.
fn render_once(
    fixture: &PipelineFixture,
    canvas_factory: &mut TinySkiaCanvasFactory,
    engines: &mut LayoutEngines,
) {
    let child_errors = std::collections::HashMap::new();
    let mut pipeline = RenderPipeline::new();
    let Some(diff) = pipeline.diff(&fixture.module, fixture.diff_input()) else {
        return;
    };
    let mut ctx = layout_context!(fixture, &child_errors, canvas_factory, engines);
    black_box(pipeline.layout(diff, &mut ctx));
}

fn bench_pipeline(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("pipeline");
    let fixture = PipelineFixture::new();
    let mut canvas_factory = TinySkiaCanvasFactory::new();

    group.bench_function("full_render_pipeline_iteration", |bencher| {
        let mut engines = LayoutEngines::default();
        bencher.iter(|| render_once(&fixture, &mut canvas_factory, &mut engines));
    });

    group.finish();
}

// ----------------------------------------------------------------------------
// Criterion Entry Point
// ----------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_vdom,
    bench_styling,
    bench_styling_rule_buckets,
    bench_layout,
    bench_rendering,
    bench_scripting,
    bench_pipeline
);
criterion_main!(benches);
