#!/usr/bin/env python3
"""
Performance Profile Analyzer for Cranky.
Analyzes Firefox Profiler / samply JSON.GZ profile dumps, resolving symbols,
calculating exclusive/inclusive CPU time and samples, and formatting reports for LLMs and developers.
"""

import argparse
import collections
import gzip
import json
import os
import re
import shutil
import subprocess
import sys
from typing import Any, Dict, List, Optional, Set, Tuple


def demangle_rust_name(name: str) -> str:
    """Clean up and demangle Rust symbols for readability."""
    if not name:
        return "<unknown>"

    # Common replacements for rust mangling tokens
    replacements = [
        ("$LT$", "<"),
        ("$GT$", ">"),
        ("$LP$", "("),
        ("$RP$", ")"),
        ("$C$", ", "),
        ("$u20$", " "),
        ("$u7b$", "{"),
        ("$u7d$", "}"),
        ("$u3b$", ";"),
        ("$u2b$", "+"),
        ("$u26$", "&"),
        ("$u3d$", "="),
        ("$u2e$", "."),
        ("$u2f$", "/"),
        ("$u5b$", "["),
        ("$u5d$", "]"),
        ("..", "::"),
        ("_$LT$", "<"),
    ]
    cleaned = name
    for token, repl in replacements:
        cleaned = cleaned.replace(token, repl)

    # Strip rustc compiler hash suffixes like ::h0a7cbc022ee568a5
    cleaned = re.sub(r"::h[0-9a-f]{16}\b", "", cleaned)
    # Strip llvm suffixes like (.llvm.4680320127005956729)
    cleaned = re.sub(r"\s*\(\.llvm\.[0-9]+\)", "", cleaned)

    return cleaned.strip()


class SymbolInfo:
    def __init__(
        self,
        name: str,
        file: str = "",
        line: int = 0,
        inlined_in: str = "",
        inlined_chain: Optional[List[Tuple[str, str, int]]] = None,
    ):
        self.name = name
        self.file = file
        self.line = line
        self.inlined_in = inlined_in
        self.inlined_chain = inlined_chain or []

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "file": self.file,
            "line": self.line,
            "inlined_in": self.inlined_in,
            "inlined_chain": [
                {"name": n, "file": f, "line": l} for n, f, l in self.inlined_chain
            ],
        }


class Symbolizer:
    """Batch-resolves addresses using llvm-symbolizer, eu-addr2line, or addr2line."""

    def __init__(
        self,
        binary_path: Optional[str] = None,
        preferred_tool: Optional[str] = None,
    ):
        self.binary_path = binary_path
        self.tool = preferred_tool or self._detect_tool()
        self.cache: Dict[Tuple[str, int], SymbolInfo] = {}

    def _detect_tool(self) -> Optional[str]:
        for tool in ["llvm-symbolizer", "eu-addr2line", "addr2line"]:
            if shutil.which(tool):
                return tool
        return None

    def resolve_batch(
        self, requests: List[Tuple[str, int]]
    ) -> Dict[Tuple[str, int], SymbolInfo]:
        """
        Batch resolves a list of (binary_path, address) tuples.
        Returns mapping to SymbolInfo.
        """
        results: Dict[Tuple[str, int], SymbolInfo] = {}
        unresolved_by_binary: Dict[str, List[int]] = collections.defaultdict(
            list
        )

        for bin_path, addr in requests:
            key = (bin_path, addr)
            if key in self.cache:
                results[key] = self.cache[key]
            elif bin_path and os.path.exists(bin_path) and addr >= 0:
                unresolved_by_binary[bin_path].append(addr)

        if not self.tool:
            return results

        for bin_path, addrs in unresolved_by_binary.items():
            if not addrs:
                continue
            unique_addrs = sorted(set(addrs))

            if self.tool == "llvm-symbolizer":
                self._resolve_llvm_symbolizer(bin_path, unique_addrs)
            elif self.tool in ("eu-addr2line", "addr2line"):
                self._resolve_addr2line(bin_path, unique_addrs)

        for key in requests:
            if key in self.cache:
                results[key] = self.cache[key]

        return results

    def _resolve_llvm_symbolizer(self, bin_path: str, addrs: List[int]):
        input_data = "\n".join(hex(a) for a in addrs) + "\n"
        try:
            p = subprocess.Popen(
                ["llvm-symbolizer", f"--obj={bin_path}", "-C"],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
            out, _ = p.communicate(input=input_data, timeout=15)

            # llvm-symbolizer outputs blocks separated by empty lines
            # Each block has 1 or more (func_name, file:line:col) pairs for inlined chains
            blocks = out.strip().split("\n\n")
            for idx, block in enumerate(blocks):
                if idx >= len(addrs):
                    break
                lines = [
                    line.strip() for line in block.splitlines() if line.strip()
                ]
                if not lines:
                    continue

                chain: List[Tuple[str, str, int]] = []
                for i in range(0, len(lines) - 1, 2):
                    f_name = demangle_rust_name(lines[i])
                    loc = lines[i + 1]
                    m = re.match(r"^(.*?):(\d+)(?::\d+)?$", loc)
                    f_file = (
                        m.group(1)
                        if m and m.group(1) not in ("??", "?:0:0")
                        else ""
                    )
                    f_line = int(m.group(2)) if m else 0
                    chain.append((f_name, f_file, f_line))

                if chain:
                    innermost = chain[0]
                    # Outermost caller in the inlined sequence (if inlined)
                    outermost_name = (
                        chain[-1][0]
                        if len(chain) > 1 and chain[-1][0] != innermost[0]
                        else ""
                    )
                    self.cache[(bin_path, addrs[idx])] = SymbolInfo(
                        name=innermost[0],
                        file=innermost[1],
                        line=innermost[2],
                        inlined_in=outermost_name,
                        inlined_chain=chain,
                    )
        except Exception:
            pass

    def _resolve_addr2line(self, bin_path: str, addrs: List[int]):
        input_data = "\n".join(hex(a) for a in addrs) + "\n"
        cmd = [self.tool, "-f", "-C", "-e", bin_path] if self.tool else []
        try:
            p = subprocess.Popen(
                cmd,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
            out, _ = p.communicate(input=input_data, timeout=20)
            lines = [line.strip() for line in out.strip().splitlines()]
            for i in range(0, len(lines), 2):
                addr_idx = i // 2
                if addr_idx >= len(addrs):
                    break
                func_name = demangle_rust_name(lines[i])
                loc = lines[i + 1] if i + 1 < len(lines) else "?:0"
                m = re.match(r"^(.*?):(\d+)(?::\d+)?$", loc)
                filename = (
                    m.group(1)
                    if m and m.group(1) not in ("??", "?:0", "?:0:0")
                    else ""
                )
                line_num = int(m.group(2)) if m else 0
                self.cache[(bin_path, addrs[addr_idx])] = SymbolInfo(
                    name=func_name,
                    file=filename,
                    line=line_num,
                    inlined_in="",
                    inlined_chain=[(func_name, filename, line_num)],
                )
        except Exception:
            pass


def find_profile_file(specified_path: Optional[str] = None) -> str:
    """Find profile file from argument or standard search locations."""
    if specified_path:
        if os.path.exists(specified_path):
            return specified_path
        raise FileNotFoundError(
            f"Specified profile file not found: {specified_path}"
        )

    search_candidates = [
        "scripts/profiles.json.gz",
        "scripts/profile.json.gz",
        "profile.json.gz",
        "profiles.json.gz",
        "profile.json",
        "scripts/profile.json",
    ]
    for cand in search_candidates:
        if os.path.exists(cand):
            return cand

    raise FileNotFoundError(
        f"No profile file found in default locations: {', '.join(search_candidates)}. "
        "Please provide the path as an argument."
    )


def load_profile_data(file_path: str) -> Dict[str, Any]:
    """Load JSON or GZIPPED JSON profile file."""
    if file_path.endswith(".gz"):
        with gzip.open(file_path, "rt", encoding="utf-8", errors="replace") as f:
            return json.load(f)
    else:
        with open(file_path, "r", encoding="utf-8", errors="replace") as f:
            return json.load(f)


def find_binary_path(
    profile_data: Dict[str, Any], explicit_binary: Optional[str] = None
) -> Optional[str]:
    """Identify the target binary path."""
    if explicit_binary and os.path.exists(explicit_binary):
        return explicit_binary

    # Look in profile's libs
    for lib in profile_data.get("libs", []):
        lib_path = lib.get("path", "")
        if "cranky" in lib_path and os.path.exists(lib_path):
            return lib_path

    # Fallback to local build artifacts
    for local_candidate in ["target/release/cranky", "target/debug/cranky"]:
        if os.path.exists(local_candidate):
            return local_candidate

    return None


class ProfileAnalyzer:
    def __init__(
        self,
        profile_data: Dict[str, Any],
        symbolizer: Symbolizer,
        binary_path: Optional[str] = None,
    ):
        self.profile = profile_data
        self.symbolizer = symbolizer
        self.binary_path = binary_path
        self.libs = profile_data.get("libs", [])

    def get_threads_summary(self) -> List[Dict[str, Any]]:
        threads = self.profile.get("threads", [])
        summary = []
        for idx, t in enumerate(threads):
            name = t.get("name", f"thread-{idx}")
            pid = t.get("pid")
            tid = t.get("tid")
            samples = t.get("samples", {})
            sample_count = len(samples.get("stack", []))
            cpu_deltas = samples.get("threadCPUDelta", [])
            total_cpu_ms = (
                sum(c for c in cpu_deltas if c is not None) / 1000.0
                if cpu_deltas
                else 0.0
            )

            summary.append({
                "index": idx,
                "name": name,
                "pid": pid,
                "tid": tid,
                "sample_count": sample_count,
                "cpu_ms": total_cpu_ms,
                "is_main": t.get("isMainThread", False) or name == "cranky",
            })
        return summary

    def select_threads(
        self, thread_pattern: Optional[str], all_threads: bool
    ) -> List[Tuple[int, Dict[str, Any]]]:
        threads = self.profile.get("threads", [])
        if all_threads:
            return [
                (idx, t)
                for idx, t in enumerate(threads)
                if len(t.get("samples", {}).get("stack", [])) > 0
            ]

        if thread_pattern:
            matches = []
            for idx, t in enumerate(threads):
                t_name = t.get("name", "")
                if (
                    thread_pattern.lower() in t_name.lower()
                    or str(t.get("tid")) == thread_pattern
                ):
                    matches.append((idx, t))
            if matches:
                return matches

        # Default to cranky thread, or the thread with the most samples
        for idx, t in enumerate(threads):
            if t.get("name") == "cranky":
                return [(idx, t)]

        # Fallback: thread with max samples
        max_t = max(
            enumerate(threads),
            key=lambda x: len(x[1].get("samples", {}).get("stack", [])),
        )
        return [max_t]

    def analyze_thread(
        self,
        thread: Dict[str, Any],
        top_n: int = 20,
        hide_system: bool = False,
        sort_by: str = "exclusive",
    ) -> Dict[str, Any]:
        samples = thread.get("samples", {})
        stacks = samples.get("stack", [])
        total_samples = len(stacks)
        if total_samples == 0:
            return {
                "thread_name": thread.get("name"),
                "total_samples": 0,
                "hot_functions": [],
                "call_trees": [],
            }

        weights = samples.get("weight", [1] * total_samples)
        cpu_deltas = samples.get("threadCPUDelta", [0] * total_samples)
        total_cpu_ms = (
            sum(c for c in cpu_deltas if c is not None) / 1000.0
            if cpu_deltas
            else 0.0
        )

        stack_prefixes = thread.get("stackTable", {}).get("prefix", [])
        stack_frames = thread.get("stackTable", {}).get("frame", [])
        frame_funcs = thread.get("frameTable", {}).get("func", [])
        frame_addrs = thread.get("frameTable", {}).get("address", [])
        func_names = thread.get("funcTable", {}).get("name", [])
        func_resources = thread.get("funcTable", {}).get("resource", [])
        resource_libs = thread.get("resourceTable", {}).get("lib", [])
        strings = thread.get("stringArray", [])

        # Collect symbolization requests
        sym_requests: List[Tuple[str, int]] = []
        frame_to_lib_path: Dict[int, str] = {}

        for f_idx in range(len(frame_funcs)):
            func_idx = frame_funcs[f_idx]
            res_idx = (
                func_resources[func_idx]
                if func_idx < len(func_resources)
                else -1
            )
            lib_idx = (
                resource_libs[res_idx]
                if res_idx >= 0 and res_idx < len(resource_libs)
                else None
            )
            lib_path = ""
            if (
                lib_idx is not None
                and lib_idx >= 0
                and lib_idx < len(self.libs)
            ):
                lib_path = self.libs[lib_idx].get("path", "")

            # If lib_path is cranky or empty and we have binary_path
            if (not lib_path or "cranky" in lib_path) and self.binary_path:
                lib_path = self.binary_path

            frame_to_lib_path[f_idx] = lib_path
            addr = frame_addrs[f_idx] if f_idx < len(frame_addrs) else -1
            if isinstance(addr, int) and addr > 0 and lib_path:
                sym_requests.append((lib_path, addr))

        # Batch resolve symbols
        symbol_map = self.symbolizer.resolve_batch(sym_requests)

        # Unwind stacks & aggregate metrics
        exclusive_samples: Dict[int, int] = collections.Counter()
        inclusive_samples: Dict[int, int] = collections.Counter()
        exclusive_cpu: Dict[int, float] = collections.Counter()
        inclusive_cpu: Dict[int, float] = collections.Counter()

        # Track callers and callees per frame (for hot tree analysis)
        callers: Dict[int, collections.Counter] = collections.defaultdict(
            collections.Counter
        )
        callees: Dict[int, collections.Counter] = collections.defaultdict(
            collections.Counter
        )

        for s_idx, stack_id in enumerate(stacks):
            if stack_id is None:
                continue
            w = weights[s_idx] if s_idx < len(weights) else 1
            cpu = (
                (cpu_deltas[s_idx] or 0) / 1000.0
                if s_idx < len(cpu_deltas)
                else 0.0
            )

            curr = stack_id
            call_stack: List[int] = []
            while curr is not None and curr >= 0 and curr < len(stack_frames):
                f_idx = stack_frames[curr]
                call_stack.append(f_idx)
                curr = stack_prefixes[curr]

            if not call_stack:
                continue

            # Leaf frame gets exclusive credit
            leaf_frame = call_stack[0]
            exclusive_samples[leaf_frame] += w
            exclusive_cpu[leaf_frame] += cpu

            seen_in_stack: Set[int] = set()
            for depth, f_idx in enumerate(call_stack):
                if f_idx not in seen_in_stack:
                    seen_in_stack.add(f_idx)
                    inclusive_samples[f_idx] += w
                    inclusive_cpu[f_idx] += cpu

                # Record caller -> callee relation (call_stack is leaf-to-root)
                if depth + 1 < len(call_stack):
                    caller_frame = call_stack[depth + 1]
                    callers[f_idx][caller_frame] += w
                    callees[caller_frame][f_idx] += w

        def get_frame_info(f_idx: int) -> Dict[str, Any]:
            func_idx = frame_funcs[f_idx] if f_idx < len(frame_funcs) else -1
            addr = frame_addrs[f_idx] if f_idx < len(frame_addrs) else -1
            lib_path = frame_to_lib_path.get(f_idx, "")
            lib_name = os.path.basename(lib_path) if lib_path else "unknown"

            raw_name = ""
            if func_idx >= 0 and func_idx < len(func_names):
                str_idx = func_names[func_idx]
                if str_idx < len(strings):
                    raw_name = strings[str_idx]

            # Check resolved symbol
            resolved: Optional[SymbolInfo] = (
                symbol_map.get((lib_path, addr))
                if isinstance(addr, int) and addr > 0
                else None
            )

            if resolved and resolved.name and resolved.name != "??":
                name = resolved.name
                file_path = resolved.file
                line_no = resolved.line
                inlined_in = resolved.inlined_in
                inlined_chain = resolved.inlined_chain
            else:
                name = (
                    demangle_rust_name(raw_name)
                    if raw_name
                    else (hex(addr) if isinstance(addr, int) else "?")
                )
                file_path = ""
                line_no = 0
                inlined_in = ""
                inlined_chain = []

            # Categorize system frame
            is_system = False
            if lib_name in (
                "libc.so.6",
                "ld-linux-x86-64.so.2",
                "libpthread.so.0",
                "[vdso]",
            ):
                is_system = True
            elif (
                name.startswith("__")
                or name.startswith("syscall")
                or "sys_futex" in name
                or name.startswith("0x")
            ):
                is_system = True

            return {
                "frame_id": f_idx,
                "name": name,
                "raw_name": raw_name,
                "addr": hex(addr) if isinstance(addr, int) and addr >= 0 else None,
                "library": lib_name,
                "file": file_path,
                "line": line_no,
                "inlined_in": inlined_in,
                "inlined_chain": inlined_chain,
                "is_system": is_system,
            }

        # Build list of unique frames
        all_active_frames = set(exclusive_samples.keys()) | set(
            inclusive_samples.keys()
        )
        frames_data = []

        for f_idx in all_active_frames:
            info = get_frame_info(f_idx)
            if hide_system and info["is_system"]:
                continue

            ex_s = exclusive_samples[f_idx]
            in_s = inclusive_samples[f_idx]
            ex_cpu = exclusive_cpu[f_idx]
            in_cpu = inclusive_cpu[f_idx]

            info.update({
                "exclusive_samples": ex_s,
                "exclusive_percent": (
                    (ex_s / total_samples) * 100.0 if total_samples else 0.0
                ),
                "exclusive_cpu_ms": ex_cpu,
                "inclusive_samples": in_s,
                "inclusive_percent": (
                    (in_s / total_samples) * 100.0 if total_samples else 0.0
                ),
                "inclusive_cpu_ms": in_cpu,
            })
            frames_data.append(info)

        # Sort frames
        if sort_by == "inclusive":
            frames_data.sort(
                key=lambda x: (x["inclusive_samples"], x["exclusive_samples"]),
                reverse=True,
            )
        elif sort_by == "cpu":
            frames_data.sort(
                key=lambda x: (x["exclusive_cpu_ms"], x["exclusive_samples"]),
                reverse=True,
            )
        else:  # default: exclusive
            frames_data.sort(
                key=lambda x: (x["exclusive_samples"], x["inclusive_samples"]),
                reverse=True,
            )

        hot_functions = frames_data[:top_n]

        # Build hot call trees for the top 5 functions
        call_trees = []
        for hot in hot_functions[:5]:
            f_id = hot["frame_id"]

            # Top callers
            top_callers = []
            for c_id, count in callers[f_id].most_common(4):
                c_info = get_frame_info(c_id)
                top_callers.append({
                    "name": c_info["name"],
                    "file": c_info["file"],
                    "line": c_info["line"],
                    "library": c_info["library"],
                    "inlined_in": c_info.get("inlined_in", ""),
                    "samples": count,
                    "percent": (
                        (count / hot["inclusive_samples"] * 100.0)
                        if hot["inclusive_samples"]
                        else 0.0
                    ),
                })

            # Top callees
            top_callees = []
            for callee_id, count in callees[f_id].most_common(4):
                callee_info = get_frame_info(callee_id)
                top_callees.append({
                    "name": callee_info["name"],
                    "file": callee_info["file"],
                    "line": callee_info["line"],
                    "library": callee_info["library"],
                    "inlined_in": callee_info.get("inlined_in", ""),
                    "samples": count,
                    "percent": (
                        (count / hot["inclusive_samples"] * 100.0)
                        if hot["inclusive_samples"]
                        else 0.0
                    ),
                })

            call_trees.append({
                "target": hot,
                "top_callers": top_callers,
                "top_callees": top_callees,
            })

        return {
            "thread_name": thread.get("name"),
            "pid": thread.get("pid"),
            "tid": thread.get("tid"),
            "total_samples": total_samples,
            "total_cpu_ms": total_cpu_ms,
            "hot_functions": hot_functions,
            "call_trees": call_trees,
        }


def format_markdown_report(
    profile_path: str,
    binary_path: Optional[str],
    threads_summary: List[Dict[str, Any]],
    analyzed_threads: List[Dict[str, Any]],
) -> str:
    """Generate structured markdown report optimized for LLMs and developers."""
    lines = []
    lines.append("# Profile Analysis Report")
    lines.append("")
    lines.append(f"- **Profile Source:** `{profile_path}`")
    lines.append(
        f"- **Target Binary:** `{binary_path or 'Not detected / stripped'}`"
    )
    lines.append("")
    lines.append("## 🧵 Thread Overview")
    lines.append("")
    lines.append("| Thread Name | PID/TID | Samples | Est. CPU Time | Share |")
    lines.append("| :--- | :--- | :--- | :--- | :--- |")

    total_all_samples = sum(t["sample_count"] for t in threads_summary)
    for t in threads_summary:
        if t["sample_count"] == 0:
            continue
        share = (
            (t["sample_count"] / total_all_samples * 100.0)
            if total_all_samples
            else 0.0
        )
        lines.append(
            f"| `{t['name']}` | `{t['pid']}` / `{t['tid']}` |"
            f" {t['sample_count']:,} | {t['cpu_ms']:.2f} ms | {share:.1f}% |"
        )
    lines.append("")

    for t_data in analyzed_threads:
        t_name = t_data["thread_name"]
        total_s = t_data["total_samples"]
        total_cpu = t_data["total_cpu_ms"]
        hot_funcs = t_data["hot_functions"]

        lines.append(
            f"## 🔥 Hotspots: Thread `{t_name}` ({total_s:,} samples,"
            f" {total_cpu:.2f} ms CPU)"
        )
        lines.append("")
        if not hot_funcs:
            lines.append("_No samples found for this thread._\n")
            continue

        lines.append("### Top Hot Functions")
        lines.append("")
        lines.append(
            "| # | Exclusive (Self) | Inclusive | CPU (Self) | Function |"
            " Inlined In | Source Location | Library |"
        )
        lines.append(
            "| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |"
        )

        for idx, f in enumerate(hot_funcs, 1):
            ex_str = f"{f['exclusive_samples']} ({f['exclusive_percent']:.1f}%)"
            in_str = f"{f['inclusive_samples']} ({f['inclusive_percent']:.1f}%)"
            cpu_str = f"{f['exclusive_cpu_ms']:.2f} ms"
            loc_str = f"`{f['file']}:{f['line']}`" if f["file"] else "-"
            lib_str = f"`{f['library']}`" if f["library"] else "-"
            name = f["name"]
            inlined_str = f"`{f['inlined_in']}`" if f.get("inlined_in") else "-"

            lines.append(
                f"| {idx} | **{ex_str}** | {in_str} | {cpu_str} | `{name}` |"
                f" {inlined_str} | {loc_str} | {lib_str} |"
            )

        lines.append("")

        # Call trees section
        call_trees = t_data.get("call_trees", [])
        if call_trees:
            lines.append("### 🌲 Call Trees for Top Hotspots")
            lines.append("")
            for item in call_trees:
                target = item["target"]
                target_name = target["name"]
                loc = (
                    f" (`{target['file']}:{target['line']}`)"
                    if target["file"]
                    else ""
                )
                inlined_note = (
                    f" [inlined in `{target['inlined_in']}`]"
                    if target.get("inlined_in")
                    else ""
                )
                lines.append(f"#### `{target_name}`{inlined_note}{loc}")
                lines.append(
                    f"- **Self Time:** {target['exclusive_samples']} samples"
                    f" ({target['exclusive_percent']:.1f}%),"
                    f" {target['exclusive_cpu_ms']:.2f} ms CPU"
                )
                lines.append(
                    f"- **Inclusive:** {target['inclusive_samples']} samples"
                    f" ({target['inclusive_percent']:.1f}%)"
                )

                if item["top_callers"]:
                    lines.append("- **Top Callers (Ancestors):**")
                    for c in item["top_callers"]:
                        c_loc = (
                            f" (`{c['file']}:{c['line']}`)"
                            if c["file"]
                            else ""
                        )
                        c_inlined = (
                            f" [inlined in `{c['inlined_in']}`]"
                            if c.get("inlined_in")
                            else ""
                        )
                        lines.append(
                            f"  - 🔺 `{c['name']}`{c_inlined}{c_loc} —"
                            f" {c['samples']} samples ({c['percent']:.1f}%)"
                        )

                if item["top_callees"]:
                    lines.append("- **Top Callees (Children):**")
                    for c in item["top_callees"]:
                        c_loc = (
                            f" (`{c['file']}:{c['line']}`)"
                            if c["file"]
                            else ""
                        )
                        c_inlined = (
                            f" [inlined in `{c['inlined_in']}`]"
                            if c.get("inlined_in")
                            else ""
                        )
                        lines.append(
                            f"  - 🔻 `{c['name']}`{c_inlined}{c_loc} —"
                            f" {c['samples']} samples ({c['percent']:.1f}%)"
                        )
                lines.append("")

    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(
        description="Analyze Firefox Profiler / samply JSON.GZ profile dumps."
    )
    parser.add_argument(
        "profile",
        nargs="?",
        help="Path to profile JSON.GZ (auto-detected if omitted)",
    )
    parser.add_argument(
        "--binary",
        "-b",
        help="Path to compiled binary for addr2line symbol resolution",
    )
    parser.add_argument(
        "--thread",
        "-t",
        help="Filter by specific thread name or TID (defaults to cranky)",
    )
    parser.add_argument(
        "--all-threads", action="store_true", help="Analyze all active threads"
    )
    parser.add_argument(
        "--top",
        "-n",
        type=int,
        default=20,
        help="Number of top functions to display (default: 20)",
    )
    parser.add_argument(
        "--hide-system",
        action="store_true",
        help="Hide system/libc/kernel runtime frames",
    )
    parser.add_argument(
        "--sort-by",
        choices=["exclusive", "inclusive", "cpu"],
        default="exclusive",
        help="Sort order (default: exclusive)",
    )
    parser.add_argument(
        "--symbolizer",
        choices=[
            "auto",
            "llvm-symbolizer",
            "eu-addr2line",
            "addr2line",
            "none",
        ],
        default="auto",
        help="Symbolizer tool (default: auto)",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output machine-readable JSON format instead of Markdown",
    )

    args = parser.parse_args()

    try:
        profile_path = find_profile_file(args.profile)
    except FileNotFoundError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)

    profile_data = load_profile_data(profile_path)
    binary_path = find_binary_path(profile_data, args.binary)

    preferred_tool = (
        None
        if args.symbolizer == "auto"
        else (None if args.symbolizer == "none" else args.symbolizer)
    )
    symbolizer = Symbolizer(
        binary_path=binary_path, preferred_tool=preferred_tool
    )
    if args.symbolizer == "none":
        symbolizer.tool = None

    analyzer = ProfileAnalyzer(profile_data, symbolizer, binary_path)
    threads_summary = analyzer.get_threads_summary()
    selected_threads = analyzer.select_threads(args.thread, args.all_threads)

    analyzed_threads = [
        analyzer.analyze_thread(
            t,
            top_n=args.top,
            hide_system=args.hide_system,
            sort_by=args.sort_by,
        )
        for _, t in selected_threads
    ]

    if args.json:
        output_data = {
            "profile_path": profile_path,
            "binary_path": binary_path,
            "threads_summary": threads_summary,
            "analyzed_threads": analyzed_threads,
        }
        print(json.dumps(output_data, indent=2))
    else:
        report = format_markdown_report(
            profile_path, binary_path, threads_summary, analyzed_threads
        )
        print(report)


if __name__ == "__main__":
    main()
