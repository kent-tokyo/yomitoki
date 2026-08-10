# yomitoki

[![CI](https://github.com/kent-tokyo/yomitoki/actions/workflows/ci.yml/badge.svg)](https://github.com/kent-tokyo/yomitoki/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/yomitoki.svg)](https://crates.io/crates/yomitoki)
[![docs.rs](https://docs.rs/yomitoki/badge.svg)](https://docs.rs/yomitoki)
[![License](https://img.shields.io/crates/l/yomitoki.svg)](#许可协议)

[English](README.md) | [日本語](README_ja.md) | **中文**

快速、可解释、无需路线搜索的分子可合成性诊断库。

yomitoki 是一个基于 [chematic](https://github.com/kent-tokyo/chematic) 构建的、快速、可解释、route-free(无需逆合成路线搜索)的分子可合成性诊断库。

yomitoki 不仅仅返回一个单一的合成可及性分数,而是读取分子结构,解释一个分子为何看起来易于合成或难以合成。它会指出支撑该评估的结构性证据,并报告这一判断的可信程度。

名称来自日文词汇「読み解き」(yomitoki),意为仔细审视某事物并揭示其含义 — 这正是本库的核心职责:不是改造分子,而是读懂并解释它。

> yomitoki 不仅仅是估算可合成性,它还揭示了该估算背后的证据与推理过程。

> **状态:`0.1.0-alpha.1` 已发布到 crates.io,但本仓库当前内容领先于该版本。** 计划中的六个组件已全部实现(包括 `fragment_rarity`)。不过 `fragment_rarity` 是可选启用的 —— yomitoki 本身不附带 fragment-frequency 语料库(AGENTS.md §5.4 禁止将语料库作为巨型二进制文件直接嵌入库中),除非构建(`tools/build-fragment-corpus/`)并配置(`AnalysisConfig.fragment_model`)一个语料库,否则该组件保持未启用状态。已发布的 `0.1.0-alpha.1` 早于此变更 —— 变更内容请参见 [`CHANGELOG.md`](CHANGELOG.md)。这是一个预发布版本,在正式 `0.1.0` 之前公开 API 仍可能变化。当前范围及尚未实现的部分请参见 [`docs/architecture.md`](docs/architecture.md)。

## 生态定位

```text
chematic    分子表示与化学信息学
    |
yomitoki    读取并解释分子的可合成性
    |
renkin      规划逆合成路线
```

yomitoki 从不执行路线搜索 — 这不是 v0.1 阶段的范围限制,而是永久性的职责边界。详见下方“yomitoki 不做什么”。

## yomitoki 做什么

* 解析分子(通过 `chematic`),返回结构化的 `SynthesizabilityReport`,而不是单一数值。
* 将评估分解为独立的组件(ring topology、size/topology、stereochemical burden、functional-group liability、input quality/applicability、fragment rarity —— 最后一项为可选启用,详见下方"局限性")。
* 将 **score**(可合成性/难度)、**confidence**(判断的可信度)与 **applicability**(该分子是否在模型的适用范围内)分为不同字段 — 难以合成的分子不会因此自动被判定为低置信度。
* 输出机器可读的 finding code 与结构化 evidence,而非仅有文字说明。
* 从不运行逆合成搜索。yomitoki 仅对分子本身进行评估,不会为其规划合成路线。
* 提供 `yomitoki` 命令行工具,支持单分子及批量(`.sdf`/SMILES 文件)分析 — 详见下方“命令行界面”。
* `analyze_batch(&[Molecule], &AnalysisConfig) -> Vec<Result<...>>` — 无需经过 CLI 或文件格式,为库调用方提供同样保证输入顺序的批处理入口。

## yomitoki 不做什么

* 逆合成规划、反应模板应用、前体生成、路线排序 — 这些是 [RENKIN](https://github.com/kent-tokyo/renkin) 的职责。
* 分子解析、环感知(ring perception)、芳香性判定、立体化学指认 — 这些是 [chematic](https://github.com/kent-tokyo/chematic) 的职责,yomitoki 仅调用它。
* 毒性预测、SDS/危险品分类、产率预测、成本预测。
* v0.1 不追求完整元素周期表覆盖或有机金属化合物的完整支持。

## 快速开始

```rust
use yomitoki::{analyze_smiles, AnalysisConfig};

let config = AnalysisConfig::default();
let report = analyze_smiles("C1CC2CCC1C2", &config)?; // 降冰片烷 (norbornane)

println!("{:?}", report.overall.verdict);
println!("difficulty = {}", report.overall.difficulty.value());
println!("confidence = {}", report.overall.confidence.value());

for finding in &report.findings {
    println!("{:?}: {}", finding.code, finding.explanation);
}
```

运行完整示例:

```bash
cargo run --example basic
```

## 命令行界面

```bash
yomitoki analyze "C1CC2CCC1C2" --format json
yomitoki analyze --input molecules.sdf --format jsonl --output reports.jsonl
```

* `yomitoki analyze "<SMILES>" [--format human|json|jsonl]` — 分析作为参数传入的单个分子。
* `yomitoki analyze --input <file> [--format human|json|jsonl] [--output <file>]` — 批处理模式。`<file>` 可以是 `.sdf` 文件,也可以是每行一个 SMILES 的文件(可选择带有以空白分隔的名称列,即标准的 `.smi` 约定)。
* 批处理模式保持输入顺序,且不会因单条记录失败而中止整体处理 — 失败的记录会成为一条错误条目(JSON 中的 `"error"` 字段,或 human 格式下的 `ERROR:` 区块),而不是被跳过。只有在所有记录都处理完毕后,若存在任何失败,进程退出码才会为非零。
* `jsonl` 格式在单分子模式与批处理模式下使用相同的 `{"input", "report"|"error"}` 包装结构 — 无论以哪种方式调用,下游逐行解析器看到的都是同一套 schema。
* 退出码:`0` 表示成功,`1` 表示分子解析/分析失败(单分子模式)或批处理中至少一条记录失败,`2` 表示用法错误(参数不正确)。
* 命令行工具输出的报告同样是 `fragment_rarity: null` —— 因为 CLI 目前还没有配置语料库的方式。详见下方”局限性”。

## 报告结构示例

以下是 `cargo run --example basic` 的真实输出(截至当前组件配置)。降冰片烷(`C1CC2CCC1C2`)涉及 ring topology,其桥环还会生成一条简化建议:

```text
Verdict: ModeratelyAccessible
Synthesizability: 0.66
Confidence: 1.00
Dominant penalties:
1. Bridged ring system spanning 7 atoms — bridgehead connectivity typically increases synthetic difficulty.
Simplification suggestions (heuristic, not a guarantee):
1. ReplaceBridgedRingWithMonocyclicAnalog: Bridgehead connectivity in this ring system is a direct driver of the ring_topology contribution to difficulty. A monocyclic (or less-fused) analog, if the target application allows one, would remove this specific burden — this is a structural heuristic, not a guarantee the replacement is chemically equivalent or that synthesis actually becomes easier.
```

一个立体中心密集的片段(`CC(O)C(N)C(C)C(O)C(N)C`)同时涉及 `stereochemical_burden` 与 `functional_group_liability`(difficulty),以及 applicability 针对未指定立体化学的独立 confidence 惩罚 — 注意 difficulty 与 confidence 是独立变化的:

```text
Verdict: ModeratelyAccessible
Synthesizability: 0.74
Confidence: 0.85
Dominant penalties:
1. 4 tetrahedral stereocenter(s) (specified or unspecified) requiring synthetic control.
2. Stereocenter density 0.33 is above the 0.25 threshold — stereocenters are concentrated in a compact region, leaving little room for staged, orthogonal control.
3. Reactive/unstable functional group detected: primary amine (Brenk et al. 2008 structural alert).
Simplification suggestions (heuristic, not a guarantee):
1. ReduceStereocenterDensity: Stereocenters are concentrated in a compact region, leaving little room for staged, orthogonal stereocontrol. Reducing the number of stereocenters, or spreading them further apart in the structure, would lower this contribution to difficulty — this is a structural heuristic, not a guarantee.
```

一个环氧化物(`C1CO1`)单独涉及 `functional_group_liability` — 它直接封装了 chematic 的 Brenk et al.(2008)结构警示集:

```text
Verdict: LikelyAccessible
Synthesizability: 0.87
Confidence: 1.00
Dominant penalties:
1. Reactive/unstable functional group detected: epoxide (Brenk et al. 2008 structural alert).
```

一个 9 元环(`C1CCCCCCCC1`)涉及 `ring_topology` 的 macrocycle 分支,并生成属于它自己的简化建议:

```text
Verdict: LikelyAccessible
Synthesizability: 0.75
Confidence: 1.00
Dominant penalties:
1. Macrocyclic ring of 9 atoms (at or above the 9-atom macrocycle threshold).
Simplification suggestions (heuristic, not a guarantee):
1. SimplifyMacrocyclicClosure: Macrocyclic ring closure is a direct driver of the ring_topology contribution to difficulty (large-ring closures often need high-dilution or specialized macrocyclization methods). A smaller ring or acyclic analog, if chemically acceptable, would remove this burden — this is a structural heuristic, not a guarantee.
```

季戊四醇四乙酸酯(`CC(=O)OCC(COC(C)=O)(COC(C)=O)COC(C)=O`)拥有 4 个互不相邻的酯基环境,在 Brenk 警示之外,还会触发 `functional_group_liability` 的 "dense functionalization" 信号(`chematic::chem::identify_functional_groups`,Ertl 2017 聚类):

```text
Verdict: LikelyAccessible
Synthesizability: 0.81
Confidence: 1.00
Dominant penalties:
1. 4 distinct functional-group environments (Ertl 2017 clustering), above the 3 threshold — multiple independent reactive/functional regions can compete for reagent selectivity and complicate protecting-group strategy.
2. Reactive/unstable functional group detected: ketone alpha (Brenk et al. 2008 structural alert).
3. Reactive/unstable functional group detected: acetal ketal (Brenk et al. 2008 structural alert).
```

丙氨酸阴离子(`C[C@@H](N)C(=O)[O-]`,去质子化丙氨酸)同时具有一个已指定的立体中心*和*一个带负电荷的原子 — 后者是 chematic 的一个已知 bug([#267](https://github.com/kent-tokyo/chematic/issues/267))。程序会安全地跳过立体分析,而不是崩溃或瞎猜,并相应地降低置信度,与"未指定立体中心"的情形明确区分开:

```text
Verdict: LikelyAccessible
Synthesizability: 0.92
Confidence: 0.60
Dominant penalties:
1. Reactive/unstable functional group detected: primary amine (Brenk et al. 2008 structural alert).
2. Reactive/unstable functional group detected: acetal ketal (Brenk et al. 2008 structural alert).
3. Stereo analysis could not be run for this molecule: it contains a negatively charged atom, which triggers an arithmetic-overflow bug in chematic's stereo perception (panics in debug builds, produces an unverified result in release builds — see chematic issue #267). Stereocenter count/density and stereo completeness are unavailable, not verified to be zero/complete.
```

在未配置 fragment corpus 的情况下(默认状态 — 见下文),总体分数仍会低于配置了语料库后给出的结果。

每份报告还包含一个 `Provenance` 区块(schema 版本、yomitoki 版本、chematic 版本、ruleset 版本、fragment corpus 的模型版本、config hash),使不同版本之间的结果具有可比性 — 详见 `docs/architecture.md`。

## 组件实现状态(v0.1)

| 组件 | 状态 |
|---|---|
| `input_quality` / applicability | 已实现 |
| `ring_topology` | 已实现 |
| `size_topology` | 已实现 |
| `stereochemical_burden` | 已实现(仅四面体立体中心 — 见"局限性") |
| `functional_group_liability` | 已实现(反应性/不稳定官能团 + dense functionalization — 见"局限性") |
| `fragment_rarity` | 已实现,可选启用 —— 除非 `AnalysisConfig.fragment_model` 配置了语料库,否则为 `None`(见"局限性") |

真正未被评估的组件在 `ComponentScores` 中显示为 `None`(例如未配置语料库时的 `fragment_rarity`),而不是伪造的零分。

`suggestions: Vec<SimplificationSuggestion>` 目前覆盖 6 种代码中的 4 种(`ReplaceBridgedRingWithMonocyclicAnalog`、`SimplifyMacrocyclicClosure`、`ReduceStereocenterDensity`、`IncreaseFragmentPrecedent`)— 详见"局限性"。所有建议都仅是诊断性的、启发式的,从不宣称确定性(`expected_effect` 始终为 `MayReduceDifficulty`,从不是 `LikelyReducesDifficulty`)。

## 与现有工具的区别

* **SAscore** 将片段频率与复杂度惩罚合并为单一数值返回。yomitoki 返回按组件划分的诊断结果、置信度、适用性、证据,以及简化建议。
* **SYBA** 是一个易/难二分类器。yomitoki 则以诊断与解释为核心。
* **SCScore** 是一个学习得到的合成复杂度分数。yomitoki 则分解为透明的、具有化学意义命名的因素。
* **RAscore** 近似逆合成成功率。yomitoki 是 route-free 的,并解释其评估背后的结构性原因。
* **AiZynthFinder、ASKCOS、RENKIN** 都是路线规划器。yomitoki 从不生成合成路线。

## 与 SAscore 的比较

针对 `chematic::chem::sa_score`(Ertl & Schuffenhauer 2009)的最小化进程内比较 — 这是 AGENTS.md §27 的一项完成标准。这不是校准或准确性声明:两个分数彼此并未做过拟合,衡量的也是不同的东西(SAscore:片段频率 + 复杂度惩罚;yomitoki:按组件分解的结构性负担)。两者的量表方向也相反 — SAscore 为 `1`(容易)到 `10`(困难),yomitoki 的 `difficulty` 为 `0.0` 到 `1.0`,这里没有将两者重新缩放到同一轴上。

`cargo run --example sa_score_comparison` 的真实输出:

```text
molecule                                sa_score      yomitoki_diff  verdict
ethanol                                     3.45               0.01  LikelyAccessible
benzene                                     2.40               0.10  LikelyAccessible
norbornane (bridged)                        8.20               0.34  ModeratelyAccessible
stereocenter-dense fragment                 8.32               0.27  ModeratelyAccessible
epoxide                                     6.23               0.13  LikelyAccessible
aspirin                                     4.67               0.27  ModeratelyAccessible
paracetamol                                 4.56               0.24  LikelyAccessible
caffeine (fused heterocycle)                4.94               0.29  ModeratelyAccessible
acyl halide                                 6.62               0.09  LikelyAccessible
cyclopropane (strained)                     3.94               0.13  LikelyAccessible
nitrile                                     4.97               0.04  LikelyAccessible
alanine (specified stereocenter)            3.55               0.14  LikelyAccessible
bridged ring + several stereocenters       10.00               0.69  Challenging
spiro ring system                           5.52               0.20  LikelyAccessible
```

真正有意思的是两者出现分歧的行,而不是一致的行 — 分歧并不自动意味着 yomitoki 有 bug。最明显的例子是酰氯:SAscore 给出 `6.62`(片段少见),yomitoki 给出 `0.09`(`LikelyAccessible`)— 一种廉价、极为常见的酰化试剂,在 yomitoki 自身模型中几乎没有结构性负担。阿司匹林(`4.67` 对 `0.27`)与"局限性"中已经描述过的 Brenk 有效性缺口是同一种形状。两者大体一致的情况(咖啡因、螺环、桥环 + 多个立体中心)也不能证明其中任何一个是"正确的"— 二者都尚未针对真实合成结果进行过验证。

## 局限性

* 计划中的六个组件均已实现(见上表),但只有配置了语料库(`AnalysisConfig.fragment_model`)时,`fragment_rarity` 才会计入 `overall.difficulty`/`overall.synthesizability` —— yomitoki 本身不附带语料库(AGENTS.md §5.4)。未配置时,这两个字段仍和以前一样,只反映其余五个组件。
* 对于含有带负电荷原子(羧酸根、磺酸根、磷酸根等阴离子)的分子,立体分析(`stereo_complete` 以及整个 `stereochemical_burden`)完全无法运行 — 这是 chematic 的真实 bug([#267](https://github.com/kent-tokyo/chematic/issues/267)),不是设计选择。yomitoki 对此绝不会崩溃或瞎猜(参见 `ApplicabilityReport.stereo_uncheckable` 与 `StereoAnalysisSkipped` finding),但在上游修复之前,对这类分子确实完全没有立体化学信号。
* `size_topology` 中的可旋转键(rotatable bond)项会过度惩罚简单的、可商业购得的无支链长链分子(可旋转键很多,但合成难度几乎为零)— `fragment_rarity` 旨在通过将此类片段识别为常见/有先例的片段来纠正这一点,该组件现已实现,但它是否真的能对这一具体情况做出正确纠正,尚未在生产规模的语料库上得到实证确认(参见 `tools/build-fragment-corpus` 的现状)。详见 `docs/architecture.md` 的 "Scoring direction" 一节。
* `stereochemical_burden` 仅覆盖四面体立体中心的数量与密度。以下各项经过调查后仍未实现,原因各不相同(完整依据见 `docs/architecture.md`):
  * E/Z 双键立体化学 — chematic 其实可以直接从 SMILES 的 `/`/`\` 键方向标记指定 E/Z(不需要 2D 坐标 — 此前这里的说法有误),但仅限于输入 SMILES 中实际标注过的键。目前没有类似四面体中心 `stereo_completeness` 那样的检测器,能识别"具有立体化学意义但未标注"的双键,因此仅统计已标注的双键,衡量的其实是 SMILES 书写得有多仔细,而非真实存在多少个 E/Z 中心 —— 这与下面 atropisomerism 被否决的原因属于同一类问题,只是以另一种方式出现。
  * Atropisomerism — 直接测试了 chematic 的 `detect_atropisomers` 后予以否决:同一个分子写作 `c1ccccc1-c2ccccc2` 会被判定为 atropisomer,写作 `c1ccccc1c2ccccc2` 则不会,并且它把 *para* 位取代的联苯与真正受阻的 *ortho* 位取代联苯判定为相同结果。若直接包装使用,将违反 yomitoki 自身关于原子顺序/表示形式不变性的保证。
  * 连续立体中心、季碳邻位效应 — 二者都需要一份原子级别的立体中心候选列表(包括已指定和未指定的),而 chematic 只公开了汇总计数。若在 yomitoki 内部自行实现,将使本 crate 从"使用已验证的 chematic primitive 的消费者"变为"立体中心感知本身的所有者"—— 这是迄今为止每一个已实现组件都未曾跨越的界线。
  * meso 化合物检测 — 需要图自同构 / 拓扑对称类。chematic 内部拥有此能力(`chematic-smiles::canonical_automorphism`),但并未对外公开。
* `functional_group_liability` 覆盖反应性/不稳定官能团(直接使用 chematic 的 Brenk et al. 2008 结构警示集)以及 dense functionalization(通过 chematic 的 Ertl 2017 `identify_functional_groups`,统计彼此独立的官能团簇数量)。相互不兼容的官能团组合与保护基压力均未实现 — 与上述两项不同,这两者在 chematic 中都没有可引用、已验证的 primitive 可供依赖,手工整理其中任何一项都恰好是 AGENTS.md 所警示的"过度泛化的、化学上薄弱的规则"。化学选择性负担、多官能对称性破坏,以及难以处理的氧化态组合也均未实现 — chematic 未提供任何氧化态相关 API,因此最后一项无法实现。Brenk 的规则集最初是作为药物化学筛选库的"可取性"过滤器验证的,而非合成难度信号,因此其中一些警示会对常见、廉价且有先例的官能团产生反应 — 例如阿司匹林会触发 4 条 Brenk 警示,并被判定为 `ModeratelyAccessible`,尽管它是极易合成的分子之一。这是一个与上述可旋转键问题形状相同的已知缺口,预计通过同样的方式得到纠正 —— 同样地,`fragment_rarity` 现已实现,但这一具体情况是否真的能被纠正,尚未在没有生产规模语料库的情况下得到实证确认。dense functionalization 自身也有已知缺口:它统计的是拓扑上"互不相连"的官能团簇数量,因此一个紧密互联的多官能体系(例如葡萄糖成环的多个羟基,或稠环的 β-内酰胺)会收敛为单一簇 — 与只有一个普通官能团的分子计数相同。
* yomitoki 本身不附带 fragment corpus(AGENTS.md §5.4),因此默认情况下无法检测新颖/稀有的子结构 —— 在构建(`tools/build-fragment-corpus`)并配置语料库之前,`fragment_rarity` 始终为 `None`。是否默认附带语料库尚未决定(AGENTS.md §5.4 提出的 `yomitoki-core`/`yomitoki-models`/`yomitoki-data` 拆分,或带 feature flag 的外部文件)。
* 简化建议目前覆盖 `SuggestionCode` 6 种代码中的 4 种(桥环、macrocycle、立体中心密度,以及配置了语料库时的增加片段先例)。其余 2 种缺少可用信号:季碳邻位关系尚未在任何地方计算;`brenk_matches_detailed` 按模式而非按出现次数合并原子,因此"移除多个相似反应性基团中的一个"无法定位具体是哪一次出现。所有建议的置信度都是一个统一的固定常数(0.5),而非按建议代码区分 — 因为目前没有针对真实合成结果的校准数据。
* 在校准语料库出现之前(Phase 2 及以后),`ApplicabilityReport.domain_distance` 始终为 `None`。
* 覆盖范围仅限于精选的有机元素子集,不尝试支持完整元素周期表或有机金属化合物。
* 目前评分与阈值均为基于规则的设定,尚未针对外部基准进行验证;目前还没有校准或对比结果。

## 可复现性

给定相同的输入、相同的 `AnalysisConfig`,以及相同的 yomitoki/chematic/ruleset 版本,`analyze`/`analyze_smiles` 总是返回相同的报告 — 核心评估流程中不使用任何随机性。

## 许可协议

可在 [Apache License, Version 2.0](LICENSE-APACHE) 或 [MIT 许可证](LICENSE-MIT) 之间任选其一。

## 引用

目前还没有可供引用的论文或正式发布版本。

## 路线图

剩余计划:随库附带的 fragment corpus(`fragment_rarity` 本身已实现但为可选启用 —— 见"局限性")、针对 SAscore/RAscore/路线搜索结果的*校准*(与 SAscore 的最小化*比较*并非校准,已在上文实现),以及未来的 Python 绑定。
