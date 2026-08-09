# rensei

快速、可解释、无需路线搜索的分子可合成性诊断库。

RENSEI 是一个基于 [chematic](https://github.com/kent-tokyo/chematic) 构建的、快速、可解释、route-free(无需逆合成路线搜索)的分子可合成性诊断库。

RENSEI 不仅仅返回一个单一的合成可及性分数,而是报告一个分子为何看起来易于合成或难以合成、该判断的可信程度如何,以及哪些结构因素主导了这一结果。

> **状态:v0.1 开发中。** 目前仅实现了 `input_quality`/`applicability`、`ring_topology`、`size_topology` 和 `stereochemical_burden` 四个组件。当前范围及尚未实现的部分请参见 [`docs/architecture.md`](docs/architecture.md)。

## RENSEI 做什么

* 解析分子(通过 `chematic`),返回结构化的 `SynthesizabilityReport`,而不是单一数值。
* 将评估分解为独立的组件(目前有 ring topology、size/topology、stereochemical burden、input quality/applicability;fragment rarity 与 functional-group liabilities 计划中)。
* 将 **score**(可合成性/难度)、**confidence**(判断的可信度)与 **applicability**(该分子是否在模型的适用范围内)分为不同字段 — 难以合成的分子不会因此自动被判定为低置信度。
* 输出机器可读的 finding code 与结构化 evidence,而非仅有文字说明。
* 从不运行逆合成搜索。RENSEI 仅对分子本身进行评估,不会为其规划合成路线。

## RENSEI 不做什么

* 逆合成规划、反应模板应用、前体生成、路线排序 — 这些是 [RENKIN](https://github.com/kent-tokyo/renkin) 的职责。
* 分子解析、环感知(ring perception)、芳香性判定、立体化学指认 — 这些是 [chematic](https://github.com/kent-tokyo/chematic) 的职责,RENSEI 仅调用它。
* 毒性预测、SDS/危险品分类、产率预测、成本预测。
* v0.1 不追求完整元素周期表覆盖或有机金属化合物的完整支持。

## 快速开始

```rust
use rensei::{analyze_smiles, AnalysisConfig};

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

## 报告结构示例

以下是 `cargo run --example basic` 的真实输出(截至当前组件配置)。降冰片烷(`C1CC2CCC1C2`)仅涉及 ring topology:

```text
Verdict: ModeratelyAccessible
Synthesizability: 0.66
Confidence: 1.00
Dominant penalties:
1. Bridged ring system spanning 7 atoms — bridgehead connectivity typically increases synthetic difficulty.
```

一个立体中心密集的片段(`CC(O)C(N)C(C)C(O)C(N)C`)同时涉及 `stereochemical_burden`(difficulty)与 applicability 针对未指定立体化学的独立 confidence 惩罚 — 注意 difficulty 与 confidence 是独立变化的:

```text
Verdict: LikelyAccessible
Synthesizability: 0.78
Confidence: 0.85
Dominant penalties:
1. 4 tetrahedral stereocenter(s) (specified or unspecified) requiring synthetic control.
2. Stereocenter density 0.33 is above the 0.25 threshold — stereocenters are concentrated in a compact region, leaving little room for staged, orthogonal control.
```

由于 fragment rarity 与 functional-group liability 仍未实现,总体分数仍会低于完整版 v0.1 给出的结果。

每份报告还包含一个 `Provenance` 区块(schema 版本、rensei 版本、chematic 版本、ruleset 版本、config hash),使不同版本之间的结果具有可比性 — 详见设计规范(`AGENTS.md`)第 16 节及 `docs/architecture.md`。

## 组件实现状态(v0.1)

| 组件 | 状态 |
|---|---|
| `input_quality` / applicability | 已实现 |
| `ring_topology` | 已实现 |
| `size_topology` | 已实现 |
| `stereochemical_burden` | 已实现(仅四面体立体中心 — 见"局限性") |
| `fragment_rarity` | 尚未实现 |
| `functional_group_liability` | 尚未实现 |

尚未实现的组件在 `ComponentScores` 中显示为 `None`,而不是伪造的零分。

## 与现有工具的区别

* **SAscore** 将片段频率与复杂度惩罚合并为单一数值返回。RENSEI 返回按组件划分的诊断结果、置信度、适用性、证据,以及(未来)改进建议。
* **SYBA** 是一个易/难二分类器。RENSEI 则以诊断与解释为核心。
* **SCScore** 是一个学习得到的合成复杂度分数。RENSEI 则分解为透明的、具有化学意义命名的因素。
* **RAscore** 近似逆合成成功率。RENSEI 是 route-free 的,并解释其评估背后的结构性原因。
* **AiZynthFinder、ASKCOS、RENKIN** 都是路线规划器。RENSEI 从不生成合成路线。

## 局限性

* v0.1 目前仅实现了计划中六个组件里的四个(见上表);`overall.difficulty`/`overall.synthesizability` 目前仅反映 ring topology、size/topology 与 stereochemical burden 带来的负担。
* `size_topology` 中的可旋转键(rotatable bond)项会过度惩罚简单的、可商业购得的无支链长链分子(可旋转键很多,但合成难度几乎为零)— 这是一个已知的缺口,预计在 fragment rarity(尚未实现)将此类片段识别为常见/有先例的片段后得到纠正。详见 `docs/architecture.md` 的 "Scoring direction" 一节。
* `stereochemical_burden` 仅覆盖四面体立体中心的数量与密度。E/Z 双键立体化学、atropisomerism、连续立体中心、季碳邻位效应,以及 meso 化合物检测均未实现 — 其中 E/Z 未实现的原因是:chematic 的 E/Z 判定需要 2D 坐标,而仅基于 SMILES 的处理流程中并不具备这些坐标。
* 目前还没有 fragment-rarity 语料库,因此无法检测新颖/稀有的子结构。
* 在校准语料库出现之前(Phase 2 及以后),`ApplicabilityReport.domain_distance` 始终为 `None`。
* 覆盖范围仅限于精选的有机元素子集,不尝试支持完整元素周期表或有机金属化合物。
* 目前评分与阈值均为基于规则的设定,尚未针对外部基准进行验证;目前还没有校准或对比结果。

## 可复现性

给定相同的输入、相同的 `AnalysisConfig`,以及相同的 rensei/chematic/ruleset 版本,`analyze`/`analyze_smiles` 总是返回相同的报告 — 核心评估流程中不使用任何随机性。

## 许可协议

可在 [Apache License, Version 2.0](LICENSE-APACHE) 或 [MIT 许可证](LICENSE-MIT) 之间任选其一。

## 引用

目前还没有可供引用的论文或正式发布版本。

## 路线图

完整的分阶段开发计划请参见 `AGENTS.md`(开发规范文档):fragment rarity、functional-group-liability 组件、针对 SAscore/RAscore/路线搜索结果的校准、CLI,以及未来的 Python 绑定。
