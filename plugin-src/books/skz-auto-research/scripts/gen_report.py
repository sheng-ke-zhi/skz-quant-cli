#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
skz-auto-research 报告生成器

读 <workdir>/fetched/summary.json（由 pipeline.py 产出），生成单文件 HTML 报告。
规范见 references/report-spec.md：深酒红侧栏 + 浅色正文（红色只做点缀，别铺面）、左栏 sticky 目录、红涨绿跌。

用法
  python3 gen_report.py --workdir <dir> --out reports/skz自动投研_XXX_20260919.html \
      [--title "标题"] [--scenario auto|A|B]
"""
from __future__ import annotations

import argparse
import html
import json
import os
import re
import statistics as stats
from collections import defaultdict
from datetime import datetime, timedelta, timezone

TZ = timezone(timedelta(hours=8))
OOS_SEGMENT = "后置验证"
PLATFORM = "https://quant.shengkezhi.com/"
DOC_STRATEGY = "https://docs.shengkezhi.com/strategy"


# ------------------------------------------------------------------ helpers

def esc(x) -> str:
    return html.escape("" if x is None else str(x))


def num(x, digits=2, dash="—"):
    if x is None or x == "":
        return dash
    try:
        return f"{float(x):,.{digits}f}"
    except (TypeError, ValueError):
        return esc(x)


def pct(x, digits=2, sign=False, dash="—"):
    if x is None or x == "":
        return dash
    try:
        v = float(x) * 100
    except (TypeError, ValueError):
        return esc(x)
    s = f"{v:+.{digits}f}%" if sign else f"{v:.{digits}f}%"
    return s


def mean(xs, f=float):
    xs = [x for x in xs if x is not None]
    return stats.fmean(xs) if xs else None


def median(xs):
    xs = [x for x in xs if x is not None]
    return stats.median(xs) if xs else None


def cls_of(x) -> str:
    """红涨绿跌：仅用于方向性数值。"""
    if x is None:
        return "na"
    try:
        v = float(x)
    except (TypeError, ValueError):
        return "na"
    return "pos" if v > 0 else ("neg" if v < 0 else "na")


def get_metric(d: dict, *names):
    """指标字段是中文，且不同接口命名可能微差 → 容错取值。"""
    if not isinstance(d, dict):
        return None
    for n in names:
        if n in d and d[n] is not None:
            return d[n]
    return None


# ------------------------------------------------------------------ 解析层

def parse_mine(d: dict) -> dict | None:
    return d.get("mine") or None


def parse_routes(d: dict) -> list:
    return d.get("routes") or []


def build_candidates(d: dict) -> list:
    """把每个 experiment 的 strategies + review-matrix 后置验证行 join 成候选列表。"""
    out = []
    for exp in d.get("experiments") or []:
        exp_id = exp.get("id")
        row = exp.get("metadata") or exp.get("list_row") or {}
        ov = exp.get("overview") or {}
        ov = ov.get("overview") if isinstance(ov, dict) and "overview" in ov else ov
        ov = ov or {}
        strategies = (exp.get("strategies") or {})
        strategies = strategies.get("items") if isinstance(strategies, dict) else strategies
        strategies = strategies or []
        mx = exp.get("review_matrix") or {}
        mx_items = mx.get("items") if isinstance(mx, dict) else mx
        mx_items = mx_items or []

        by_code = defaultdict(dict)
        for r in mx_items:
            code = r.get("strategy") or r.get("code")
            seg = r.get("segment_name")
            if code and seg:
                by_code[code][seg] = r

        for s in strategies:
            code = s.get("code")
            segs = by_code.get(code, {})
            oos = segs.get(OOS_SEGMENT, {})
            train = segs.get("训练集", {})
            m = s.get("metrics") or {}
            v = s.get("verdict") or {}
            long_p = get_metric(m, "多头占比")
            short_p = get_metric(m, "空头占比")
            net = None
            if long_p is not None and short_p is not None:
                try:
                    net = float(long_p) - float(short_p)
                except (TypeError, ValueError):
                    net = None
            sharpe_oos = get_metric(oos, "夏普比率")
            sharpe_train = get_metric(train, "夏普比率")
            decay = None
            if sharpe_oos is not None and sharpe_train is not None:
                try:
                    decay = float(sharpe_oos) - float(sharpe_train)
                except (TypeError, ValueError):
                    decay = None
            out.append({
                "experiment_id": exp_id,
                "problem_code": (row.get("problem_code") or ov.get("problem_code")),
                "problem_name": (row.get("problem_name") or ov.get("problem_name")),
                "freq": row.get("freq") or ov.get("freq"),
                "dataset": row.get("dataset") or ov.get("dataset"),
                "symbols_count": row.get("symbols_count") or ov.get("symbols_count"),
                "route": s.get("route") or row.get("route"),
                "code": code,
                "model": s.get("model"),
                "factor_count": s.get("factor_count"),
                "symbol_count": s.get("symbol_count"),
                "metrics": m,
                "verdict": v,
                "oos": oos,
                "train": train,
                "sharpe_full": get_metric(m, "夏普比率"),
                "sharpe_train": sharpe_train,
                "sharpe_oos": sharpe_oos,
                "decay": decay,
                "ann_oos": get_metric(oos, "年化收益"),
                "dd_oos": get_metric(oos, "最大回撤"),
                "ann_full": get_metric(m, "年化收益"),
                "net_exposure": net,
                "long_pct": long_p,
                "short_pct": short_p,
                "trades_per_year": get_metric(m, "年化交易次数"),
                "per_trade": get_metric(m, "单笔收益"),
            })
    out.sort(key=lambda c: (c["sharpe_oos"] is None, -(c["sharpe_oos"] or 0)))
    return out


def banner_warnings(cands: list) -> list:
    """异常候选预警（未来信息/结构性套利的典型形态），只提示不下结论。"""
    warns = []
    for c in cands:
        hits = []
        if (c.get("factor_count") or 99) <= 2:
            hits.append("只用 1–2 个因子")
        s_full = c.get("sharpe_full")
        if s_full is not None and c.get("dd_oos") is not None:
            try:
                if float(s_full) > 3 and abs(float(c["dd_oos"])) < 0.05:
                    hits.append("全样本夏普 >3 且后置验证回撤 <5%")
            except (TypeError, ValueError):
                pass
        ym = (c.get("verdict") or {}).get("yearly_metrics") or []
        alphas = [y.get("alpha_return") for y in ym if y.get("alpha_return") is not None]
        if len(alphas) >= 5 and all(a > 0 for a in alphas):
            inc = all(alphas[i] <= alphas[i + 1] for i in range(len(alphas) - 1))
            if inc:
                hits.append("逐年 alpha 全为正且单调放大")
        if hits:
            warns.append((c, hits))
    return warns


# ------------------------------------------------------------------ 渲染层

CSS = """
:root{
--ink:__INK__;--lede:__LEDE__;--paper:__PAPER__;--panel:__PANEL__;--line:__LINE__;
--accent:__ACCENT__;--accent-deep:__ACCENT_DEEP__;--accent-soft:__ACCENT_SOFT__;
--accent-fade:__ACCENT_FADE__;--green:__GREEN__;--gold:__GOLD__;
--mute:__MUTE__;--chip:__CHIP__;--chip-fg:__CHIP_FG__;
--nav-a:__NAV_A__;--nav-b:__NAV_B__;--nav-fg:__NAV_FG__;--nav-dim:__NAV_DIM__;
--nav-accent:__NAV_ACCENT__;--nav-line:__NAV_LINE__;--nav-hover:__NAV_HOVER__;
--th-bg:__TH_BG__;--th-fg:__TH_FG__;--row-hover:__ROW_HOVER__;--bar-track:__BAR_TRACK__;
--warn-bg:__WARN_BG__;--ok-bg:__OK_BG__;--shadow:__SHADOW__;}
*{box-sizing:border-box}
html{scroll-behavior:smooth}
body{margin:0;color:var(--ink);background:var(--paper);
font:16px/1.6 "Iowan Old Style","Palatino Linotype",Palatino,"Songti SC",serif}
.shell{display:grid;grid-template-columns:290px 1fr;min-height:100vh}
nav{position:sticky;top:0;height:100vh;overflow:auto;padding:28px 22px 48px;color:var(--nav-fg);
background:linear-gradient(172deg,var(--nav-a) 0%,var(--nav-b) 100%)}
nav .mark{font-family:"Avenir Next","PingFang SC",sans-serif;letter-spacing:.18em;font-size:11px;
color:var(--nav-accent);text-transform:uppercase}
nav h1{font-size:20px;margin:8px 0 10px;font-weight:600;line-height:1.3;color:#fff;
padding-bottom:13px;border-bottom:1px solid var(--nav-line)}
nav p{color:var(--nav-dim);font-size:12.5px;margin:0 0 18px}
nav a{color:var(--nav-fg);text-decoration:none;display:block;position:relative;
padding:6px 8px 6px 13px;font-size:14px;border-radius:7px;opacity:.88;
transition:background .16s,color .16s,opacity .16s}
nav a::before{content:"";position:absolute;left:0;top:50%;transform:translateY(-50%);
width:2px;height:0;border-radius:2px;background:var(--nav-accent);transition:height .16s}
nav a:hover{background:var(--nav-hover);color:#fff;opacity:1}
nav a:hover::before{height:15px}
nav .sec{margin-top:16px;font-size:11px;letter-spacing:.14em;text-transform:uppercase;color:var(--nav-accent)}
main{padding:46px 54px 90px;max-width:1100px}
h2{font-size:26px;margin:46px 0 12px;padding-top:22px;
border-top:2px solid;
border-image:linear-gradient(90deg,var(--accent),var(--accent-fade) 52%,rgba(255,255,255,0) 84%) 1}
h2:first-of-type{border-top:none;border-image:none;margin-top:0}
h3{font-size:19px;margin:26px 0 8px}
h4{font-size:16px;margin:18px 0 6px}
p,li{color:var(--ink)}
.lede{font-size:18.5px;color:var(--lede);max-width:48rem}
.meta{display:flex;flex-wrap:wrap;gap:8px;margin:16px 0 26px}
.chip{font-family:"Avenir Next","PingFang SC",sans-serif;font-size:12px;background:var(--chip);
border:1px solid var(--line);border-radius:999px;padding:4px 10px;color:var(--chip-fg)}
code,.mono{font-family:"IBM Plex Mono","SF Mono",Menlo,monospace;font-size:12.5px;color:var(--accent-deep)}
.callout{background:linear-gradient(90deg,var(--accent-soft),rgba(255,255,255,0) 62%),var(--panel);
border-left:3px solid var(--accent);border-radius:0 6px 6px 0;padding:14px 16px;margin:16px 0 24px}
.callout.warn{border-left-color:var(--gold);
background:linear-gradient(90deg,var(--warn-bg),rgba(255,255,255,0) 62%),var(--panel)}
.callout.ok{border-left-color:var(--green);
background:linear-gradient(90deg,var(--ok-bg),rgba(255,255,255,0) 62%),var(--panel)}
table{width:100%;border-collapse:separate;border-spacing:0;font-size:14px;background:var(--panel);
margin:14px 0 10px;border:1px solid var(--line);border-radius:10px;overflow:hidden;
box-shadow:0 1px 3px var(--shadow)}
th,td{border-bottom:1px solid var(--line);padding:8px 10px;text-align:left;vertical-align:top}
tr:last-child td{border-bottom:none}
th{font-family:"Avenir Next","PingFang SC",sans-serif;font-size:12px;letter-spacing:.04em;
color:var(--th-fg);font-weight:700;white-space:nowrap;background:var(--th-bg)}
tbody tr:hover td{background:var(--row-hover)}
td.n,th.n{text-align:right;font-variant-numeric:tabular-nums}
.pos{color:var(--accent);font-weight:700}
.neg{color:var(--green);font-weight:700}
.na{color:var(--mute)}
.kpi{display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:12px;margin:18px 0 24px}
.kpi div{background:var(--panel);border:1px solid var(--line);border-radius:8px;padding:12px 14px;
box-shadow:0 1px 3px var(--shadow)}
.kpi b{display:block;font-size:22px;font-family:"Avenir Next","PingFang SC",sans-serif;color:var(--accent-deep)}
.kpi span{font-size:12px;color:var(--mute)}
.notes{font-size:13.5px;color:var(--mute);margin:6px 0 20px}
ul.tight{margin:8px 0 20px;padding-left:20px}
ul.tight li{margin:5px 0}
.bar{height:8px;background:var(--bar-track);position:relative;min-width:120px;border-radius:999px;overflow:hidden}
.bar i{position:absolute;left:0;top:0;bottom:0;border-radius:999px;background:var(--accent)}
footer{margin-top:60px;padding-top:18px;font-size:12.5px;color:var(--mute);
border-top:1px solid var(--line)}
@media(max-width:900px){.shell{grid-template-columns:1fr}nav{position:static;height:auto}
main{padding:28px 20px 60px}}
"""


# ------------------------------------------------------------------ 主题（配色层）
# 结构在上面的 CSS 里，颜色全在这里。改配色只动这一块，不要改 CSS 结构。
DEFAULT_THEME = "scarlet"

THEMES = {
    # 备选：深酒红侧栏 + 近白正文，红只做点缀
    "crimson": {
        "label": "酒红 · 深侧栏",
        "INK": "#1f1a1b", "LEDE": "#3d3536", "PAPER": "#faf8f7", "PANEL": "#ffffff",
        "LINE": "#ece4e2", "ACCENT": "#c8102e", "ACCENT_DEEP": "#8f0a20",
        "ACCENT_SOFT": "#fdf4f5", "ACCENT_FADE": "rgba(200,16,46,.22)",
        "GREEN": "#12795c", "GOLD": "#b3813c", "MUTE": "#7d7273",
        "CHIP": "#f7f2f2", "CHIP_FG": "#8d5f66",
        "NAV_A": "#2c1118", "NAV_B": "#471a24", "NAV_FG": "#e9dfdd", "NAV_DIM": "#b0949b",
        "NAV_ACCENT": "#c99a9f", "NAV_LINE": "rgba(255,255,255,.11)",
        "NAV_HOVER": "rgba(255,255,255,.07)",
        "TH_BG": "#fbf6f6", "TH_FG": "#4a4043", "ROW_HOVER": "#fdf9f9", "BAR_TRACK": "#f1eaea",
        "WARN_BG": "#fdf6ec", "OK_BG": "#f1f8f6", "SHADOW": "rgba(31,26,27,.04)",
    },
    # 宣纸暖底 + 墨褐侧栏 + 朱砂红（东方、文气、耐看）
    "vermilion": {
        "label": "朱砂 · 宣纸",
        "INK": "#1f1a16", "LEDE": "#3e352c", "PAPER": "#fbf7f0", "PANEL": "#ffffff",
        "LINE": "#eae0d1", "ACCENT": "#bc3a2c", "ACCENT_DEEP": "#8c2a1f",
        "ACCENT_SOFT": "#fbf0eb", "ACCENT_FADE": "rgba(188,58,44,.20)",
        "GREEN": "#2f6b52", "GOLD": "#b0824a", "MUTE": "#7b7266",
        "CHIP": "#f6f1e7", "CHIP_FG": "#8c5a44",
        "NAV_A": "#241f18", "NAV_B": "#332b22", "NAV_FG": "#efe6da", "NAV_DIM": "#a99d8b",
        "NAV_ACCENT": "#cf9a72", "NAV_LINE": "rgba(255,255,255,.12)",
        "NAV_HOVER": "rgba(255,255,255,.07)",
        "TH_BG": "#faf5ec", "TH_FG": "#4d4237", "ROW_HOVER": "#fdfaf4", "BAR_TRACK": "#efe7db",
        "WARN_BG": "#fbf3e5", "OK_BG": "#f0f7f3", "SHADOW": "rgba(31,26,20,.05)",
    },
    # 近白底 + 深玫瑰棕侧栏 + 柔和珊瑚红（明亮、轻、现代）
    "coral": {
        "label": "珊瑚玫瑰 · 明亮",
        "INK": "#241b1c", "LEDE": "#443536", "PAPER": "#fff9f8", "PANEL": "#ffffff",
        "LINE": "#f3e3e1", "ACCENT": "#e0655b", "ACCENT_DEEP": "#b4453e",
        "ACCENT_SOFT": "#fdefec", "ACCENT_FADE": "rgba(224,101,91,.24)",
        "GREEN": "#1c8a6b", "GOLD": "#d99a5b", "MUTE": "#8b7979",
        "CHIP": "#fdf1ef", "CHIP_FG": "#b2605a",
        "NAV_A": "#2c1f23", "NAV_B": "#412d33", "NAV_FG": "#f6eae7", "NAV_DIM": "#c2a5a5",
        "NAV_ACCENT": "#f2a79d", "NAV_LINE": "rgba(255,255,255,.12)",
        "NAV_HOVER": "rgba(255,255,255,.08)",
        "TH_BG": "#fef4f2", "TH_FG": "#6b4c4c", "ROW_HOVER": "#fffafa", "BAR_TRACK": "#f7e9e6",
        "WARN_BG": "#fdf3e7", "OK_BG": "#eff8f4", "SHADOW": "rgba(36,27,28,.05)",
    },
    # 当前默认：冷灰白底 + 近黑中性侧栏 + 清晰正红（编辑感、克制、锐利）
    "scarlet": {
        "label": "正红 · 冷灰（当前默认）",
        "INK": "#17181a", "LEDE": "#2f3134", "PAPER": "#f6f6f4", "PANEL": "#ffffff",
        "LINE": "#e5e5e1", "ACCENT": "#cc2632", "ACCENT_DEEP": "#96161f",
        "ACCENT_SOFT": "#fdeef0", "ACCENT_FADE": "rgba(204,38,50,.22)",
        "GREEN": "#0f7a5c", "GOLD": "#a8834a", "MUTE": "#74757a",
        "CHIP": "#f1f1ee", "CHIP_FG": "#6d4a4e",
        "NAV_A": "#17181b", "NAV_B": "#24262a", "NAV_FG": "#e8e8ea", "NAV_DIM": "#9a9ca2",
        "NAV_ACCENT": "#e0797f", "NAV_LINE": "rgba(255,255,255,.12)",
        "NAV_HOVER": "rgba(255,255,255,.08)",
        "TH_BG": "#faf8f8", "TH_FG": "#4a4b4f", "ROW_HOVER": "#fafafa", "BAR_TRACK": "#eceaea",
        "WARN_BG": "#fbf4e9", "OK_BG": "#eef7f3", "SHADOW": "rgba(23,24,26,.05)",
    },
    # 砖红侧栏 + 米白正文（红做主角但仍压低饱和）
    "brick": {
        "label": "砖红 · 白底",
        "INK": "#20191a", "LEDE": "#40322f", "PAPER": "#fdfaf7", "PANEL": "#ffffff",
        "LINE": "#eee0dc", "ACCENT": "#b8432f", "ACCENT_DEEP": "#8a2d1d",
        "ACCENT_SOFT": "#fbeee9", "ACCENT_FADE": "rgba(184,67,47,.22)",
        "GREEN": "#2c6a52", "GOLD": "#b8863f", "MUTE": "#7d6f6b",
        "CHIP": "#f8efeb", "CHIP_FG": "#8d5a49",
        "NAV_A": "#7a2229", "NAV_B": "#55161c", "NAV_FG": "#f7e6e2", "NAV_DIM": "#d3a9a6",
        "NAV_ACCENT": "#f0b9a8", "NAV_LINE": "rgba(255,255,255,.16)",
        "NAV_HOVER": "rgba(255,255,255,.10)",
        "TH_BG": "#fbf3ef", "TH_FG": "#4c3b36", "ROW_HOVER": "#fdf8f6", "BAR_TRACK": "#f1e5e0",
        "WARN_BG": "#fbf2e6", "OK_BG": "#eff7f4", "SHADOW": "rgba(32,25,26,.05)",
    },
}


def render_css(theme: str = None) -> str:
    """把主题色值灌进 CSS 模板。未知主题名回落到 DEFAULT_THEME。"""
    key = theme if theme in THEMES else DEFAULT_THEME
    css = CSS
    for k, v in THEMES[key].items():
        if k == "label":
            continue
        css = css.replace("__%s__" % k, v)
    left = re.findall(r"__[A-Z_]+__", css)
    if left:
        raise ValueError("CSS 模板有未替换的占位符：%s" % sorted(set(left)))
    return css


def theme_list() -> str:
    return " / ".join("%s(%s)" % (k, v["label"]) for k, v in THEMES.items())


def sec(id_, title):
    return f'<h2 id="{id_}">{esc(title)}</h2>'


def funnel_html(funnel, kpi) -> str:
    if not funnel:
        return ""
    rows = []
    base = None
    for f in funnel:
        rem = f.get("remaining")
        if base is None and rem:
            base = float(rem)
        w = 100.0 * float(rem) / base if (base and rem is not None) else 0
        rows.append(
            f'<tr><td>{esc(f.get("stage"))}</td><td class="n">{num(rem,0)}</td>'
            f'<td class="n">{num(f.get("eliminated"),0)}</td>'
            f'<td style="width:32%"><div class="bar"><i style="width:{w:.1f}%"></i></div></td>'
            f'<td class="n">{w:.1f}%</td></tr>')
    return ('<table><thead><tr><th>漏斗环节</th><th class="n">剩余</th><th class="n">本环节淘汰</th>'
            '<th>占初始比</th><th class="n">留存</th></tr></thead><tbody>'
            + "".join(rows) + "</tbody></table>")


def build_html(d: dict, title: str, scenario: str, theme: str = None) -> str:
    css = render_css(theme)
    mine = parse_mine(d)
    routes = parse_routes(d)
    cands = build_candidates(d)
    exps = d.get("experiments") or []
    warn = banner_warnings(cands)
    gen_at = d.get("generated_at") or datetime.now(TZ).isoformat(timespec="seconds")

    # ---- 摘要
    bullets = []
    if mine:
        rounds = mine.get("rounds") or []
        ok_rounds = [r for r in rounds if r.get("retained") is not None]
        rets = [r.get("retained") for r in ok_rounds]
        bullets.append(
            f"因子挖掘共触发 <b>{len(rounds)}</b> 轮，"
            f"其中 <b>{len(ok_rounds)}</b> 轮拿到有效产出；停止原因："
            f"<b>{esc(mine.get('stopped_reason') or '—')}</b>。")
        if rets:
            bullets.append(
                f"单轮有效因子数（终选保留）中位 <b>{median(rets):.0f}</b> 个、"
                f"区间 {min(rets):.0f}–{max(rets):.0f} 个（阈值 {esc(mine.get('threshold'))}）。")
    if routes:
        r0 = routes[0]
        tops = [(t or {}).get("sharpe") for t in (r0.get("top_factors") or [])]
        tops = [t for t in tops if t is not None]
        bullets.append(
            f"路线《{esc(r0.get('route_name'))}》累计因子 <b>{num(r0.get('factor_count'),0)}</b> 个，"
            f"历史最优因子夏普 <b>{(num(max(tops)) if tops else '—')}</b>。")
    if exps:
        values = [e.get('metadata') or e.get('list_row') or {} for e in exps]
        n_bt = sum(r['n_backtests'] for r in values) if all(r.get('n_backtests') is not None for r in values) else None
        n_pass = sum(r['passed'] for r in values) if all(r.get('passed') is not None for r in values) else None
        candidate_count = len(cands) if all(e.get('strategies') is not None for e in exps) else None
        bullets.append(
            f"策略探索 <b>{len(exps)}</b> 次，累计回测 <b>{num(n_bt,0)}</b> 条、通过 <b>{num(n_pass,0)}</b> 条，"
            f"候选接口实读 <b>{num(candidate_count,0)}</b> 条（接口缺失时不补零）。")
    if cands:
        so = [c["sharpe_oos"] for c in cands]
        bullets.append(
            f"候选的<b>后置验证段</b>夏普中位 <b>{num(median(so))}</b>；"
            f"全周期夏普中位 {num(median([c['sharpe_full'] for c in cands]))}"
            f"（两者口径不同，排序一律用前者）。")

    pipeline = d.get('pipeline') or {}
    if pipeline:
        bullets.insert(0, f"流水线结果：<b>{esc(pipeline.get('outcome'))}</b>；"
                       f"{esc(pipeline.get('failure_reason') or '全部任务已确认终态并完成抓取')}。")
        bullets.append(f"累计尝试 <b>{len(pipeline.get('attempt_ledger') or [])}</b> 次；"
                       f"写前保留费用 <b>{esc(pipeline.get('reserved_cent'))}</b> 分，"
                       f"上限 {esc(pipeline.get('cap_cent'))} 分；含失败/未知/补跑，非实际扣费账单。")
    if pipeline.get('data_gaps'):
        bullets.append('数据缺口：<code>' + esc(json.dumps(pipeline['data_gaps'], ensure_ascii=False))
                       + '</code>；缺失字段显示“—”，不使用 remaining_count 替代 strategy_count。')
    summary_html = "".join(f"<li>{b}</li>" for b in bullets) or "<li>无数据。</li>"

    # ---- KPI
    kpis = []
    if mine:
        rounds = mine.get("rounds") or []
        rets = [r.get("retained") for r in rounds if r.get("retained") is not None]
        kpis += [("挖掘轮次", f"{len(rounds)}"), ("有效因子中位", f"{median(rets):.0f}" if rets else "—")]
    if routes:
        kpis += [("路线累计因子", num(routes[0].get("factor_count"), 0))]
    if exps:
        kpis += [("探索次数", str(len(exps)))]
    kpis += [("候选接口实读", str(len(cands)) if all(e.get('strategies') is not None for e in exps) else '—'),
             ("后置验证夏普中位", num(median([c["sharpe_oos"] for c in cands])))]
    kpi_html = "".join(f"<div><b>{esc(v)}</b><span>{esc(k)}</span></div>" for k, v in kpis)

    # ---- ③ 挖掘
    mine_html = ""
    if mine:
        rows = []
        for r in mine.get("rounds") or []:
            ret = r.get("retained")
            flag = ""
            if r.get("status") == "low_retain":
                flag = '<span class="neg">触发早停</span>'
            elif r.get("status") == "error":
                flag = '<span class="neg">平台异常</span>'
            rows.append(
                f'<tr><td class="n">{esc(r.get("round"))}</td>'
                f'<td class="n">{num(r.get("total_candidates"),0)}</td>'
                f'<td class="n">{num(ret,0)}</td>'
                f'<td class="n">{pct(r.get("retain_rate"))}</td>'
                f'<td>{esc(r.get("status"))}</td><td>{flag}</td></tr>')
        mine_html += ('<table><thead><tr><th class="n">轮次</th><th class="n">候选数</th>'
                      '<th class="n">有效因子数</th><th class="n">留存率</th><th>状态</th><th>备注</th>'
                      '</tr></thead><tbody>' + "".join(rows) + "</tbody></table>")
        mine_html += (f'<p class="notes">“有效因子数” = 该次挖掘的终选保留数（<code>mining runs.retained</code> / '
                      f'<code>mining overview.kpi.retained</code>）。阈值 {esc(mine.get("threshold"))} '
                      f'仅为“筛掉明显低产轮次”的下限，不是质量门槛——判断高低要与同账号历史分布比。</p>')
        for r in (mine.get("rounds") or []):
            if r.get("funnel"):
                mine_html += f'<h3>第 {esc(r.get("round"))} 轮漏斗</h3>'
                mine_html += funnel_html(r.get("funnel"), r.get("kpi"))
                eb = r.get("elimination_breakdown") or []
                if eb:
                    mine_html += ('<p class="notes">淘汰分解：'
                                  + "；".join(f'{esc(x.get("reason"))} {num(x.get("count"),0)}'
                                              for x in eb) + '</p>')
        if routes:
            mx = max([r.get("factor_count") or 0 for r in routes] + [1])
            rrows_parts = []
            for r in routes:
                tops = [t.get("sharpe") for t in (r.get("top_factors") or [])
                        if t.get("sharpe") is not None]
                best = max(tops) if tops else None
                width = 100.0 * (r.get("factor_count") or 0) / mx
                rrows_parts.append(
                    f'<tr><td><code>{esc(r.get("route_code"))}</code></td>'
                    f'<td>{esc(r.get("route_name"))}</td>'
                    f'<td class="n">{num(r.get("factor_count"),0)}</td>'
                    f'<td style="width:26%"><div class="bar"><i style="width:{width:.1f}%"></i></div></td>'
                    f'<td class="n">{num(best)}</td></tr>')
            rrows = "".join(rrows_parts)
            mine_html += ('<h3>路线因子库存</h3><table><thead><tr><th>路线 code</th><th>名称</th>'
                          '<th class="n">累计因子</th><th>相对量</th><th class="n">历史最优因子夏普</th>'
                          '</tr></thead><tbody>' + rrows + "</tbody></table>")
            mine_html += ('<p class="notes">“历史最优因子夏普” = 该路线出过的最好因子（'
                          '<code>factor summary.top_factors[].sharpe</code>）。'
                          '切勿拿 <code>agg.best_sharpe</code> 当因子真实夏普——那是 '
                          '多问题、多模型评估里的最大值，属多重比较产物；历史最优值也不能当真实夏普。</p>')

    # ---- ④ 探索
    exp_html = ""
    if exps:
        rows = []
        for e in exps:
            row = e.get("metadata") or e.get("list_row") or {}
            ov = e.get("overview") or {}
            ov = ov.get("overview") if isinstance(ov, dict) and "overview" in ov else (ov or {})
            pr = row.get("pass_rate")
            n_s = row.get("strategy_count")
            n_bt = row.get("n_backtests")
            n_p = row.get("passed")
            strategies = e.get('strategies')
            actual_count = (len(strategies.get('items', [])) if isinstance(strategies, dict)
                            else (len(strategies) if isinstance(strategies, list) else None))
            rows.append(
                f'<tr><td>{esc(row.get("problem_name") or ov.get("problem_name") or "—")}</td>'
                f'<td><code>{esc(row.get("problem_code") or ov.get("problem_code") or "—")}</code></td>'
                f'<td><code>{esc(e.get("route_code") or row.get("route") or "—")}</code></td>'
                f'<td>{esc(row.get("freq") or ov.get("freq") or "—")}</td>'
                f'<td class="n">{num(row.get("symbols_count") or ov.get("symbols_count"),0)}</td>'
                f'<td class="n">{num(n_bt,0)}</td><td class="n">{num(n_p,0)}</td>'
                f'<td class="n">{pct(pr)}</td><td class="n">{num(n_s,0)}</td>'
                f'<td class="n">{num(actual_count,0)}</td>'
                f'<td class="n">{num(row.get("total_elapsed") or ov.get("total_elapsed"),0)}</td></tr>')
        exp_html += ('<table><thead><tr><th>研究问题</th><th>code</th><th>路线 code</th><th>频率</th>'
                     '<th class="n">标的数</th><th class="n">回测数</th><th class="n">历史通过</th>'
                     '<th class="n">通过率</th><th class="n">平台候选数</th><th class="n">候选接口实读</th>'
                     '<th class="n">耗时(s)</th></tr></thead><tbody>' + "".join(rows) + "</tbody></table>")
        exp_html += ('<p class="notes">三个数字别混：<b>回测数</b>（<code>n_backtests</code>，全部回测，'
                     '不随候选消费改写）、<b>历史通过</b>（<code>passed</code>）、'
                     '<b>待审核候选</b>（<code>strategy_count</code>，保存入库或删除后会减少）。'
                     '且 <code>passed + failed ≠ n_backtests</code>（存在一批未评级回测），'
                     '不要据此推断失败率。</p>')

    # ---- ⑤ 候选
    cand_html = ""
    passed_values = [(e.get('metadata') or e.get('list_row') or {}).get('passed') for e in exps]
    passed_sum = sum(passed_values) if all(v is not None for v in passed_values) else None
    mismatch_html = ""
    if passed_sum is not None and passed_sum > 0 and not cands and all(e.get('strategies') is not None for e in exps):
        mismatch_html = (
            '<div class="callout warn"><b>通过数与待审核候选数不一致</b>：'
            f'平台记录本次回测通过 <b>{passed_sum}</b> 条，但候选接口'
            '（<code>experiment strategies</code>）返回 <b>0</b> 条。'
            '实测这是平台常见现象——两种可能：候选未持久化，或已被审核消费/清空，'
            '<b>CLI 侧无法区分</b>。因此汇总通过率请用 <code>overview.passed</code>，'
            '评审用 <code>strategies</code> 的实际条数，差异在此显式说明。</div>')
    elif cands and passed_sum is not None and passed_sum > len(cands):
        mismatch_html = (
            '<div class="callout warn">平台记录通过 '
            f'<b>{passed_sum}</b> 条，候选接口返回 <b>{len(cands)}</b> 条。'
            '候选未持久化或已被审核消费/清空均可能导致差异，CLI 侧不能确定原因。</div>')
    if cands:
        rows = []
        for i, c in enumerate(cands, 1):
            rows.append(
                f'<tr><td class="n">{i}</td><td>{esc(c.get("problem_name") or "—")}</td>'
                f'<td><code>{esc(c.get("code"))}</code></td><td>{esc(c.get("model") or "—")}</td>'
                f'<td class="n">{num(c.get("factor_count"),0)}</td>'
                f'<td class="n">{num(c.get("sharpe_full"))}</td>'
                f'<td class="n">{num(c.get("sharpe_train"))}</td>'
                f'<td class="n"><b>{num(c.get("sharpe_oos"))}</b></td>'
                f'<td class="n {cls_of(c.get("decay"))}">{num(c.get("decay"))}</td>'
                f'<td class="n {cls_of(c.get("ann_oos"))}">'
                f'{pct(c.get("ann_oos"), sign=True)}</td>'
                f'<td class="n {cls_of(c.get("dd_oos"))}">'
                f'{pct(c.get("dd_oos"), sign=True)}</td>'
                f'<td class="n {cls_of(c.get("net_exposure"))}">{num(c.get("net_exposure"),3)}</td></tr>')
        cand_html += ('<table><thead><tr><th class="n">#</th><th>研究问题</th><th>候选 code</th><th>模型</th>'
                      '<th class="n">因子数</th><th class="n">全周期夏普</th>'
                      '<th class="n">训练集夏普</th><th class="n">后置验证夏普</th>'
                      '<th class="n">衰减</th><th class="n">后置验证年化</th>'
                      '<th class="n">后置验证回撤</th><th class="n">净敞口</th></tr></thead><tbody>'
                      + "".join(rows) + "</tbody></table>")
        cand_html += ('<p class="notes">排序键是<b>后置验证夏普</b>（来自 <code>review-matrix</code> 的'
                      '「后置验证」段）。<b>全周期夏普禁止用于排序</b>——<code>experiment strategies</code> '
                      '的 <code>metrics</code> 覆盖本次研究全窗口，不能替代后置验证。'
                      '净敞口 = 多头占比 − 空头占比；|净敞口| &lt; 0.1 表示多空基本对称，'
                      '现金可作明确标注的参考基准，但净敞口低不证明市场风险中性，'
                      '未核实风险暴露时不能宣称风险调整后的超额收益。绿=负、红=正。</p>')

    # ---- 预警
    warn_html = ""
    if warn:
        lis = "".join(f'<li><code>{esc(c.get("code"))}</code>（{esc(c.get("problem_name") or "—")}）：'
                      f'{"；".join(h)}</li>' for c, h in warn[:20])
        warn_html = ('<div class="callout warn"><b>需要人工核验的异常形态</b>'
                     '（先怀疑未来函数/结构性套利，不要当结论）：<ul class="tight">'
                     + lis + '</ul>提示：核验方式包括查因子是否用到当根收盘、是否在换月/停牌处取值异常。</div>')

    # ---- 局限
    oos_sharpes = [c["sharpe_oos"] for c in cands]
    dec = [c["decay"] for c in cands if c["decay"] is not None]
    limits = [
        "全周期指标 ≠ 样本外。本批候选全周期夏普中位 "
        f"{num(median([c['sharpe_full'] for c in cands]))}、后置验证段中位 {num(median(oos_sharpes))}"
        + (f"，衰减中位 {num(median(dec))}" if dec else "")
        + "；排序与结论一律以后置验证段为准。",
        "基准口径：可核实的候选净敞口中位 "
        f"{num(median([c['net_exposure'] for c in cands if c['net_exposure'] is not None]),3)}，"
        "；净敞口低不证明市场风险中性。现金仅为参考基准，敞口缺失时不可验证，未计算风险调整后超额收益。",
        "多重检验偏差：候选是从全部回测里筛出来的一小撮，头部极值天然抬高；"
        "后置验证只有一段，无法复验。",
        "成本口径未知：平台未在研究问题里约定手续费/滑点，<b>单笔收益是否已扣费未知</b>，"
        "高频档尤其敏感。",
        "“后置验证”未必是真样本外：若该段止期早于当前日期，其信息量已打折；"
        "真正样本外要等入库后看 <code>nav.oos_start</code> 之后的表现。",
        "验证期长度以本次问题时间分段为准；有限样本不能证明真实夏普为正。",
        "选择偏差：本报告的分母是“回测数”与“待审核候选数”，"
        "只看通过候选会把整体水平系统性抬高。",
        "路线逻辑是自然语言（<code>key_inspect</code>），引擎实现口径未必与文字描述完全一致。",
    ]
    lim_html = "<ul class='tight'>" + "".join(f"<li>{x}</li>" for x in limits) + "</ul>"

    problem_rows = []
    for problem in d.get('problems') or []:
        segments = problem.get('time_segments') or []
        window = '；'.join(f"{s.get('name')}: {s.get('sdt')} → {s.get('edt')}" for s in segments)
        problem_rows.append(f"<tr><td><code>{esc(problem.get('code'))}</code></td>"
                            f"<td>{esc(problem.get('dataset'))}</td><td>{esc(problem.get('freq'))}</td>"
                            f"<td>{esc(window or '时间分段缺失，不能核实后置验证窗口')}</td></tr>")
    if problem_rows:
        lim_html += ('<h3>本次问题与验证时间分段</h3><table><thead><tr><th>问题</th><th>数据集</th>'
                     '<th>频率</th><th>时间分段（平台 readback）</th></tr></thead><tbody>'
                     + ''.join(problem_rows) + '</tbody></table>')

    # ---- 导航
    nav_items = [("s1", "① 结论摘要"), ("s2", "② 本次做了什么"), ("s3", "③ 因子挖掘结果"),
                 ("s4", "④ 策略探索结果"), ("s5", "⑤ 候选明细与样本外"), ("s6", "⑥ 局限与风险"),
                 ("s7", "⑦ 下一步与审核"), ("s8", "附录 · 口径与数据")]
    nav_html = "".join(f'<a href="#{i}">{esc(t)}</a>' for i, t in nav_items)

    route_code = (mine or {}).get("route") or (routes[0].get("route_code") if routes else None) \
        or ((exps[0].get("list_row") or {}).get("route") if exps else None)

    what = [
        f"场景：{esc({'A': 'A · 交易经验/策略复现', 'B': 'B · 新研究问题'}.get(scenario, scenario))}",
        f"因子路线：<code>{esc(route_code or '—')}</code>",
        f"研究问题：{'、'.join(sorted({esc(c.get('problem_code') or '—') for c in cands})) or '（见第 ④ 章）'}",
        f"数据来源：胜可知智能投研平台 <code>skz</code> CLI（只读接口取数）",
        f"状态目录：<code>{esc(d.get('workdir') or '—')}</code>",
        f"生成时间：{esc(gen_at)}",
    ]
    what_html = "<ul class='tight'>" + "".join(f"<li>{x}</li>" for x in what) + "</ul>"

    body = f"""<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>{esc(title)}</title>
<style>{css}</style>
</head>
<body>
<div class="shell">
<nav>
  <div class="mark">SKZ · AUTO RESEARCH</div>
  <h1>{esc(title)}</h1>
  <p>胜可知智能投研平台 · 自动化投研汇总</p>
  <div class="sec">目录</div>
  {nav_html}
</nav>
<main>
  <p class="lede">本报告汇总了一轮自动化投研的<b>因子挖掘</b>与<b>策略探索</b>结果，
  所有数字来自平台 CLI 只读接口；候选策略需到平台「策略审核」页人工审核后才可保存入库。</p>
  <div class="meta">
    <span class="chip">生成 {esc(gen_at)}</span>
    <span class="chip">路线 {esc(route_code or "—")}</span>
    <span class="chip">待审核候选 {len(cands)}</span>
    <span class="chip">平台 quant.shengkezhi.com</span>
  </div>

  {sec("s1", "① 结论摘要")}
  <ul class="tight">{summary_html}</ul>
  <div class="kpi">{kpi_html}</div>

  {sec("s2", "② 本次做了什么")}
  {what_html}

  {sec("s3", "③ 因子挖掘结果")}
  {mine_html or '<p class="notes">本次没有挖掘环节（场景 B：直接使用既有因子路线）。</p>'}

  {sec("s4", "④ 策略探索结果")}
  {exp_html or '<p class="notes">无探索记录。</p>'}

  {sec("s5", "⑤ 候选明细与样本外")}
  {mismatch_html}
  {cand_html or ('<p class="notes">候选接口读取不完整，无法确认候选数量。</p>' if any(e.get('strategies') is None for e in exps) else '<p class="notes">候选接口实读为 0 条。</p>')}
  {warn_html}

  {sec("s6", "⑥ 局限与风险")}
  {lim_html}

  {sec("s7", "⑦ 下一步与审核")}
  <div class="callout ok">
    <b>到胜可知平台「策略管理 → 策略审核」页逐条审核候选</b>：
    <a href="{PLATFORM}">{PLATFORM}</a>（功能说明 <a href="{DOC_STRATEGY}">{DOC_STRATEGY}</a>）
    <ul class="tight">
      <li>审核要点：不要只挑历史收益最高的——核对<b>交易成本、验证期稳定性、回撤</b>，
      以及表现<b>是否符合原路线的逻辑</b>。</li>
      <li>审核通过后需<b>单独</b>保存入库；入库后是 <code>暂停</code> 态，不会自动交易。</li>
      <li>切换「实盘」是之后的独立决定，建议先观察研究期之后的样本外表现。</li>
      <li>平台与平台文档：<a href="{PLATFORM}">{PLATFORM}</a> ·
      <a href="https://docs.shengkezhi.com/">docs.shengkezhi.com</a></li>
    </ul>
  </div>

  {sec("s8", "附录 · 口径与数据")}
  <table>
    <thead><tr><th>概念</th><th>口径</th></tr></thead>
    <tbody>
      <tr><td>有效因子数</td><td>单次挖掘的终选保留数（<code>mining runs.retained</code> / <code>mining overview.kpi.retained</code>）</td></tr>
      <tr><td>留存率</td><td><code>retain_rate</code> = retained ÷ total_candidates，平台直出，勿自算</td></tr>
      <tr><td>全周期夏普</td><td><code>experiment strategies.metrics.夏普比率</code>，窗口以本次问题配置为准，<b>不是样本外</b></td></tr>
      <tr><td>后置验证夏普</td><td><code>experiment review-matrix</code> 中 <code>segment_name=="{OOS_SEGMENT}"</code> 的行</td></tr>
      <tr><td>净敞口</td><td>多头占比 − 空头占比；现金仅为参考基准，低净敞口不证明市场风险中性</td></tr>
      <tr><td>待审核候选</td><td><code>experiment strategies</code> 的返回条数（只含通过且未入库的）</td></tr>
    </tbody>
  </table>
  <footer>
    原始数据：<code>{esc((d.get("workdir") or "") + "/fetched/summary.json")}</code><br />
    报告由 skz-auto-research 技能生成 · 数字口径见附录 · 本报告不构成投资建议
  </footer>
</main>
</div>
</body>
</html>
"""
    return body


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--workdir")
    ap.add_argument("--out")
    ap.add_argument("--title")
    ap.add_argument("--scenario", default="auto", choices=["auto", "A", "B"])
    ap.add_argument("--theme", default=DEFAULT_THEME, choices=sorted(THEMES.keys()),
                    help="配色主题：" + theme_list())
    ap.add_argument("--list-themes", action="store_true", help="只打印可选主题，不生成报告")
    a = ap.parse_args()

    if a.list_themes:
        for k, v in THEMES.items():
            flag = "  ← 默认" if k == DEFAULT_THEME else ""
            print(f"  {k:<12} {v['label']}{flag}")
        return

    if not a.workdir or not a.out:
        ap.error("--workdir 与 --out 为必填")

    src = os.path.join(a.workdir, "fetched", "summary.json")
    with open(src, encoding="utf-8") as f:
        d = json.load(f)

    scenario = a.scenario
    if scenario == "auto":
        scenario = "A" if d.get("mine") else "B"

    if a.title:
        title = a.title
    elif scenario == "A":
        r = (parse_routes(d) or [{}])[0]
        title = f"交易经验复现 · {r.get('route_name') or '自动化投研'} · 挖掘与探索汇总"
    else:
        r = (parse_routes(d) or [{}])[0]
        title = f"因子路线探索 · {r.get('route_name') or '自动化投研'} · 汇总报告"

    html_out = build_html(d, title, scenario, a.theme)
    out_dir = os.path.dirname(os.path.abspath(a.out))
    os.makedirs(out_dir, exist_ok=True)
    with open(a.out, "w", encoding="utf-8") as f:
        f.write(html_out)

    # charset 自查
    head = html_out[:1024].lower()
    ok_doctype = html_out.lstrip().lower().startswith("<!doctype html>")
    ok_charset = "charset=utf-8" in head or "charset=\"utf-8\"" in head
    print(f"✅ 生成 {a.out}  ({len(html_out)/1024:.1f} KB)")
    print(f"   doctype@开头={ok_doctype}  charset@前1024B={ok_charset}  "
          f"候选={len(build_candidates(d))} 探索={len(d.get('experiments') or [])}")
    if not (ok_doctype and ok_charset):
        raise SystemExit("❌ charset/doctype 自查未通过")


if __name__ == "__main__":
    main()
