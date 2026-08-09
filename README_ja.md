# rensei

高速で説明可能な、経路探索を伴わない分子合成容易性診断ライブラリ。

RENSEIは、[chematic](https://github.com/kent-tokyo/chematic)上に構築された、高速・説明可能・route-free(経路探索を伴わない)な分子合成容易性診断ライブラリです。

単一の合成容易性スコアだけを返すのではなく、RENSEIはその分子がなぜ作りやすい/作りにくいと判断されたのか、その判断がどの程度信頼できるのか、そしてどの構造的要因が結果を支配しているのかを報告します。

> **ステータス: v0.1開発中。** 現時点で実装済みなのは`input_quality`/`applicability`、`ring_topology`、`size_topology`の3コンポーネントのみです。現在のスコープと未実装部分については[`docs/architecture.md`](docs/architecture.md)を参照してください。

## RENSEIがすること

* 分子を(`chematic`経由で)パースし、単一の数値ではなく構造化された`SynthesizabilityReport`を返す。
* 評価を独立したコンポーネントに分解する(現時点ではring topology、size/topology、input quality/applicability。stereochemical burden、fragment rarity、functional-group liabilitiesは今後実装予定)。
* **score**(合成容易性/困難性)、**confidence**(その判断の信頼性)、**applicability**(その分子がそもそもモデルの適用範囲内かどうか)を別々のフィールドとして分離する — 作りにくい分子だからといって自動的に低confidenceになるわけではない。
* 単なる文章ではなく、機械可読なfinding codeと構造化されたevidenceを出力する。
* retrosynthesis探索を一切実行しない。RENSEIは分子単体を評価するのみで、それを作るための経路を計画することはしない。

## RENSEIがしないこと

* Retrosynthesisプランニング、reaction template適用、precursor生成、route ranking — これらは[RENKIN](https://github.com/kent-tokyo/renkin)の役割です。
* 分子パース、ring perception、aromaticity、stereochemistry割り当て — これらは[chematic](https://github.com/kent-tokyo/chematic)の役割であり、RENSEIはそれを利用するだけです。
* 毒性判定、SDS/危険物分類、収率予測、コスト予測。
* v0.1では全元素対応やorganometallic(有機金属)への完全対応は行いません。

## クイックスタート

```rust
use rensei::{analyze_smiles, AnalysisConfig};

let config = AnalysisConfig::default();
let report = analyze_smiles("C1CC2CCC1C2", &config)?; // ノルボルナン

println!("{:?}", report.overall.verdict);
println!("difficulty = {}", report.overall.difficulty.value());
println!("confidence = {}", report.overall.confidence.value());

for finding in &report.findings {
    println!("{:?}: {}", finding.code, finding.explanation);
}
```

完全なサンプルを実行する:

```bash
cargo run --example basic
```

## レポートの形式

ノルボルナン(`C1CC2CCC1C2`)に対する`cargo run --example basic`の実際の出力(このコンポーネント構成時点のもの):

```text
Verdict: ModeratelyAccessible
Synthesizability: 0.66
Confidence: 1.00
Dominant penalties:
1. Bridged ring system spanning 7 atoms — bridgehead connectivity typically increases synthetic difficulty.
```

現時点では`ring_topology`と`size_topology`のみが`difficulty`に寄与しているため、完全なv0.1(stereochemical burdenやfragment rarityも寄与する状態)が同じ分子に対して出すスコアより低めに出ます。

各レポートには`Provenance`ブロック(schema version、rensei version、chematic version、ruleset version、config hash)も含まれており、バージョン間で結果を比較できるようになっています — 設計仕様書(`AGENTS.md`)の§16および`docs/architecture.md`を参照してください。

## コンポーネントの実装状況(v0.1)

| コンポーネント | 状態 |
|---|---|
| `input_quality` / applicability | 実装済み |
| `ring_topology` | 実装済み |
| `size_topology` | 実装済み |
| `stereochemical_burden` | 未実装 |
| `fragment_rarity` | 未実装 |
| `functional_group_liability` | 未実装 |

未実装のコンポーネントは`ComponentScores`内で`None`として表現され、捏造されたゼロスコアとしては表現されません。

## 既存ツールとの違い

* **SAscore**はフラグメント頻度と複雑性ペナルティを単一の数値として返します。RENSEIはコンポーネントごとの診断、confidence、applicability、evidence、(将来的には)改善提案を返します。
* **SYBA**はeasy/hardの二値分類器です。RENSEIは診断と説明を主眼としたツールです。
* **SCScore**は学習済みの合成複雑性スコアです。RENSEIは代わりに透明で化学的に名前の付いた要因へ分解します。
* **RAscore**はretrosynthesisの成功確率を近似します。RENSEIはroute-freeであり、その評価の構造的理由を説明します。
* **AiZynthFinder、ASKCOS、RENKIN**はroute plannerです。RENSEIは経路を生成しません。

## 制限事項

* v0.1では計画されている6コンポーネントのうち3つのみを実装しています(上の表を参照)。`overall.difficulty`/`overall.synthesizability`は現状ring topologyとsize/topologyのburdenのみを反映しています。
* `size_topology`のrotatable bond(回転可能結合)に関する項は、市販されている単純な非分岐長鎖分子(回転可能結合は多いが合成難易度はほぼゼロ)を過大評価します — これは既知のギャップであり、fragment rarity(未実装)がそうしたフラグメントを一般的/前例のあるものと認識することで補正される予定です。`docs/architecture.md`の "Scoring direction" セクションを参照してください。
* fragment-rarityのコーパスがまだ存在しないため、新規/希少な部分構造は検出されません。
* `ApplicabilityReport.domain_distance`は、キャリブレーション用コーパスが存在するまで(Phase 2以降)常に`None`です。
* 対応範囲は厳選されたorganic元素のサブセットに限定されており、全元素対応やorganometallicへの対応は試みていません。
* スコアと閾値はルールベースであり、これまで外部ベンチマークに対する検証は行われていません。キャリブレーションや比較結果はまだ存在しません。

## 再現性

同一の入力、同一の`AnalysisConfig`、同一のrensei/chematic/rulesetバージョンであれば、`analyze`/`analyze_smiles`は常に同じレポートを返します — コアの評価処理には乱数を一切使用していません。

## ライセンス

[Apache License, Version 2.0](LICENSE-APACHE)または[MITライセンス](LICENSE-MIT)のいずれかの下でライセンスされています(選択可能)。

## 引用

現時点で論文や引用可能なリリースは存在しません。

## ロードマップ

フェーズ分けされた開発計画の全体については`AGENTS.md`(開発仕様書)を参照してください: stereochemical-burdenおよびfunctional-group-liabilityコンポーネント、fragment rarity、SAscore/RAscore/route-search outcomeに対するキャリブレーション、CLI、そして将来的にはPythonバインディング。
