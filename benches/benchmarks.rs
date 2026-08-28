use criterion::{Criterion, black_box, criterion_group, criterion_main};

use cranky::features::layout_engine::adapters::taffy::TaffyLayoutAdapter;
use cranky::features::layout_engine::domain::{
    AlignItems, BoxMargin, FlexDirection, JustifyContent, StyledNode,
};
use cranky::features::layout_engine::ports::LayoutEnginePort;
use cranky::features::module_runtime::domain::render_pipeline::{LayoutContext, RenderPipeline};
use cranky::features::module_runtime::ports::{AnyModulePort, ModuleInitError};
use cranky::features::styling::adapters::lightningcss::LightningCssAdapter;
use cranky::features::styling::domain::{
    ClassName, ClassNameList, ComputedStyle, ElementQuery, PseudoClass, StyleSheetName,
};
use cranky::features::styling::ports::{CssParserPort, StyleResolverPort};
use cranky::features::vdom::adapters::DefaultVdomDiffAdapter;
use cranky::features::vdom::domain::{NodeKey, TextContent, VNode};
use cranky::features::vdom::ports::VdomDiffPort;
use cranky::shared::config::domain::{Config, EngineSelection, FontFamily, FontSize, ModuleConfig};
use cranky::shared::dbus::domain::DBusSubscription;
use cranky::shared::events::signals::{SignalHub, SignalKind};
use cranky::shared::primitives::color::{Color, DrawingColor};
use cranky::shared::primitives::geometry::{LogicalPx, Position, Scale, Size};
use cranky::shared::primitives::{FunctionName, ModuleOptions, MonitorId};
use cranky::shared::rendering::adapters::tiny_skia::TinySkiaCanvasFactory;
use cranky::shared::rendering::ports::canvas::{Canvas, CanvasFactory};
use cranky::shared::scripting::adapters::lua::LuaModule;
use cranky::shared::scripting::adapters::rhai::RhaiModule;

// ----------------------------------------------------------------------------
// Helpers for Benchmark Fixtures
// ----------------------------------------------------------------------------

fn create_test_vnode_tree(depth: usize, branching: usize, prefix: &str) -> VNode {
    if depth == 0 {
        let node_key = NodeKey::new(format!("{prefix}_leaf")).ok();
        let class_name = ClassName::new("item-label").ok();
        let classes = class_name.map(|c| ClassNameList::new(vec![c]));
        let mut node = VNode::new_text(
            TextContent::new(format!("Text {prefix}")),
            classes,
            None,
            None,
            None,
            None,
        );
        if let Some(key) = node_key {
            node = node.with_key(key);
        }
        node
    } else {
        let mut children = Vec::with_capacity(branching);
        for i in 0..branching {
            let child_prefix = format!("{prefix}_{depth}_{i}");
            children.push(create_test_vnode_tree(
                depth.saturating_sub(1),
                branching,
                &child_prefix,
            ));
        }
        let node_key = NodeKey::new(format!("{prefix}_flex")).ok();
        let class_name = ClassName::new("container-box").ok();
        let classes = class_name.map(|c| ClassNameList::new(vec![c]));
        let mut node = VNode::new_flex(children, classes, None, None, None, None);
        if let Some(key) = node_key {
            node = node.with_key(key);
        }
        node
    }
}

fn create_styled_node_tree(depth: usize, branching: usize) -> StyledNode {
    let mut style = ComputedStyle::default();
    style.set_flex_direction(FlexDirection::Row);
    style.set_justify_content(JustifyContent::SpaceBetween);
    style.set_align_items(AlignItems::Center);
    style.set_margin(BoxMargin::new(1.0, 1.0, 1.0, 1.0));

    if depth == 0 {
        StyledNode::Text {
            text: TextContent::new("Benchmark Text Content".to_string()),
            style,
            on_click: None,
            on_hover: None,
            tooltip: None,
        }
    } else {
        let mut children = Vec::with_capacity(branching);
        for _ in 0..branching {
            children.push(create_styled_node_tree(depth.saturating_sub(1), branching));
        }
        StyledNode::Flex {
            children,
            style,
            on_click: None,
            on_hover: None,
            tooltip: None,
        }
    }
}

struct StaticStyleResolver {
    style: ComputedStyle,
}

impl StyleResolverPort for StaticStyleResolver {
    fn resolve_style(&self, _query: &ElementQuery) -> ComputedStyle {
        self.style.clone()
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

    fn call_function(&mut self, _name: &FunctionName) -> Result<(), ModuleInitError> {
        Ok(())
    }
}

// ----------------------------------------------------------------------------
// 1. VDOM Benchmarks
// ----------------------------------------------------------------------------

fn bench_vdom(c: &mut Criterion) {
    let mut group = c.benchmark_group("vdom");
    let differ = DefaultVdomDiffAdapter::new();

    // Small tree: depth 1, branching 4 (~5 nodes)
    let small_tree_a = create_test_vnode_tree(1, 4, "small");
    let small_tree_b = create_test_vnode_tree(1, 4, "small_mutated");

    // Medium tree: depth 3, branching 3 (~40 nodes)
    let medium_tree_a = create_test_vnode_tree(3, 3, "med");
    let medium_tree_b = create_test_vnode_tree(3, 3, "med_mutated");

    // Large tree: depth 4, branching 3 (~120 nodes)
    let large_tree_a = create_test_vnode_tree(4, 3, "large");
    let large_tree_b = create_test_vnode_tree(4, 3, "large_mutated");

    group.bench_function("diff_small_identical", |b| {
        b.iter(|| {
            let res = differ.diff(black_box(Some(&small_tree_a)), black_box(&small_tree_a));
            black_box(res);
        });
    });

    group.bench_function("diff_small_mutated", |b| {
        b.iter(|| {
            let res = differ.diff(black_box(Some(&small_tree_a)), black_box(&small_tree_b));
            black_box(res);
        });
    });

    group.bench_function("diff_medium_mutated", |b| {
        b.iter(|| {
            let res = differ.diff(black_box(Some(&medium_tree_a)), black_box(&medium_tree_b));
            black_box(res);
        });
    });

    group.bench_function("diff_large_mutated", |b| {
        b.iter(|| {
            let res = differ.diff(black_box(Some(&large_tree_a)), black_box(&large_tree_b));
            black_box(res);
        });
    });

    group.finish();
}

// ----------------------------------------------------------------------------
// 2. Styling Benchmarks
// ----------------------------------------------------------------------------

fn bench_styling(c: &mut Criterion) {
    let mut group = c.benchmark_group("styling");
    let parser = LightningCssAdapter::new();

    let sample_css = r"
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

    let Ok(sheet_name) = StyleSheetName::new("sample") else {
        return;
    };

    group.bench_function("parse_css_stylesheet", |b| {
        b.iter(|| {
            let parsed =
                parser.parse_stylesheet(black_box(sheet_name.clone()), black_box(sample_css));
            let _ = black_box(parsed);
        });
    });

    let Ok(parsed_sheet) = parser.parse_stylesheet(sheet_name, sample_css) else {
        return;
    };

    let Ok(class_container) = ClassName::new("container-box") else {
        return;
    };
    let classes_hover = [class_container];
    let pseudos_hover = [PseudoClass::Hover];
    let query_hover = ElementQuery::new("flex", None, &classes_hover, &pseudos_hover, None);

    let Ok(class_label) = ClassName::new("item-label") else {
        return;
    };
    let classes_simple = [class_label];
    let query_simple = ElementQuery::new("text", None, &classes_simple, &[], None);

    group.bench_function("resolve_style_simple_match", |b| {
        b.iter(|| {
            let style = parsed_sheet.resolve_style(black_box(&query_simple));
            black_box(style);
        });
    });

    group.bench_function("resolve_style_pseudo_class_match", |b| {
        b.iter(|| {
            let style = parsed_sheet.resolve_style(black_box(&query_hover));
            black_box(style);
        });
    });

    group.finish();
}

// ----------------------------------------------------------------------------
// 3. Layout Engine Benchmarks
// ----------------------------------------------------------------------------

fn bench_layout(c: &mut Criterion) {
    let mut group = c.benchmark_group("layout");
    let mut canvas_factory = TinySkiaCanvasFactory::new();

    let tree_small = create_styled_node_tree(1, 4);
    let tree_medium = create_styled_node_tree(3, 3);
    let tree_large = create_styled_node_tree(4, 3);

    group.bench_function("taffy_layout_small_tree", |b| {
        let mut layout_engine = TaffyLayoutAdapter::default();
        b.iter(|| {
            let mut measurer = canvas_factory.create_text_measurer(
                Scale::new(1.0),
                FontFamily::new(String::new()),
                FontSize::new(14.0),
            );
            let res = layout_engine.calculate_layout(
                black_box(tree_small.clone()),
                &mut measurer,
                Position::new(0, 0),
            );
            let _ = black_box(res);
        });
    });

    group.bench_function("taffy_layout_medium_tree", |b| {
        let mut layout_engine = TaffyLayoutAdapter::default();
        b.iter(|| {
            let mut measurer = canvas_factory.create_text_measurer(
                Scale::new(1.0),
                FontFamily::new(String::new()),
                FontSize::new(14.0),
            );
            let res = layout_engine.calculate_layout(
                black_box(tree_medium.clone()),
                &mut measurer,
                Position::new(0, 0),
            );
            let _ = black_box(res);
        });
    });

    group.bench_function("taffy_layout_large_tree", |b| {
        let mut layout_engine = TaffyLayoutAdapter::default();
        b.iter(|| {
            let mut measurer = canvas_factory.create_text_measurer(
                Scale::new(1.0),
                FontFamily::new(String::new()),
                FontSize::new(14.0),
            );
            let res = layout_engine.calculate_layout(
                black_box(tree_large.clone()),
                &mut measurer,
                Position::new(0, 0),
            );
            let _ = black_box(res);
        });
    });

    group.finish();
}

// ----------------------------------------------------------------------------
// 4. Rendering Subsystem Benchmarks
// ----------------------------------------------------------------------------

fn bench_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("rendering");
    let mut canvas_factory = TinySkiaCanvasFactory::new();
    let size = Size::new(800, 40);
    let mut buffer = vec![0_u8; 800_usize.saturating_mul(40).saturating_mul(4)];

    let bg_color = DrawingColor::Solid(Color::new(30, 30, 46, 255));
    let border_color = DrawingColor::Solid(Color::new(137, 180, 250, 255));
    let text_color = DrawingColor::Solid(Color::new(205, 214, 244, 255));

    group.bench_function("tinyskia_draw_rect_and_border", |b| {
        b.iter(|| {
            let mut canvas = canvas_factory.create_canvas(
                black_box(&mut buffer),
                black_box(size),
                Scale::new(1.0),
                FontFamily::new(String::new()),
                FontSize::new(14.0),
            );

            canvas.draw_rect(
                LogicalPx::new(0.0),
                LogicalPx::new(0.0),
                LogicalPx::new(800.0),
                LogicalPx::new(40.0),
                bg_color.clone(),
                LogicalPx::new(4.0),
            );

            canvas.draw_border(
                Position::new(0, 0),
                size,
                border_color.clone(),
                LogicalPx::new(4.0),
                LogicalPx::new(1.0),
            );
        });
    });

    group.bench_function("tinyskia_draw_text", |b| {
        b.iter(|| {
            let mut canvas = canvas_factory.create_canvas(
                black_box(&mut buffer),
                black_box(size),
                Scale::new(1.0),
                FontFamily::new(String::new()),
                FontSize::new(14.0),
            );

            canvas.draw_text(
                black_box("Workspace 1: Web | Cranky Bar Benchmark Testing"),
                None,
                Some(FontSize::new(13.0)),
                text_color.clone(),
                Position::new(12, 10),
            );
        });
    });

    group.finish();
}

// ----------------------------------------------------------------------------
// 5. Scripting Subsystem Benchmarks
// ----------------------------------------------------------------------------

fn bench_scripting(c: &mut Criterion) {
    let mut group = c.benchmark_group("scripting");
    let monitor_id = MonitorId::new("DP-1");

    let lua_source = r#"
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

    let rhai_source = r#"
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

    let config = Config::default();
    let module_config = ModuleConfig::new(
        "test_bench".into(),
        true,
        EngineSelection::Auto,
        ModuleOptions::default(),
    );

    let mut lua_mod = LuaModule::new("test_lua".to_string(), lua_source.to_string());
    let _ = lua_mod.init(&module_config, &config);

    let Ok(mut rhai_mod) = RhaiModule::new("test_rhai".to_string(), rhai_source) else {
        return;
    };
    let _ = rhai_mod.init(&module_config, &config);

    group.bench_function("lua_module_render", |b| {
        b.iter(|| {
            let node = lua_mod.render(black_box(&monitor_id));
            black_box(node);
        });
    });

    group.bench_function("rhai_module_render", |b| {
        b.iter(|| {
            let node = rhai_mod.render(black_box(&monitor_id));
            black_box(node);
        });
    });

    group.finish();
}

// ----------------------------------------------------------------------------
// 6. Full Render Pipeline Benchmark
// ----------------------------------------------------------------------------

fn bench_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline");
    let monitor_id = MonitorId::new("DP-1");

    let vnode = create_test_vnode_tree(2, 3, "pipe");
    let module = BenchMockModule { vnode };
    let vdom_diff = DefaultVdomDiffAdapter::new();
    let mut computed_style = ComputedStyle::default();
    computed_style.set_flex_direction(FlexDirection::Row);
    computed_style.set_justify_content(JustifyContent::SpaceBetween);
    computed_style.set_align_items(AlignItems::Center);
    let style_resolver = StaticStyleResolver {
        style: computed_style,
    };

    let mut canvas_factory = TinySkiaCanvasFactory::new();

    group.bench_function("full_render_pipeline_iteration", |b| {
        let mut layout_engine = TaffyLayoutAdapter::default();
        b.iter(|| {
            let mut pipeline = RenderPipeline::new();

            let diff_opt = pipeline.diff(&monitor_id, &module, &vdom_diff, None, None);

            if let Some(diff) = diff_opt {
                let mut ctx = LayoutContext {
                    style_resolver: &style_resolver,
                    current_bounds: None,
                    current_child_sizes: None,
                    canvas_factory: &mut canvas_factory,
                    layout_engine: &mut layout_engine,
                };
                let layout_res = pipeline.layout(&monitor_id, diff, &mut ctx);
                black_box(layout_res);
            }
        });
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
    bench_layout,
    bench_rendering,
    bench_scripting,
    bench_pipeline
);
criterion_main!(benches);
