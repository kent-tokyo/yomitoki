# rensei

[![CI](https://github.com/kent-tokyo/rensei/actions/workflows/ci.yml/badge.svg)](https://github.com/kent-tokyo/rensei/actions/workflows/ci.yml)

高速で説明可能な、経路探索を伴わない分子合成容易性診断ライブラリ。

RENSEIは、[chematic](https://github.com/kent-tokyo/chematic)上に構築された、高速・説明可能・route-free(経路探索を伴わない)な分子合成容易性診断ライブラリです。

単一の合成容易性スコアだけを返すのではなく、RENSEIはその分子がなぜ作りやすい/作りにくいと判断されたのか、その判断がどの程度信頼できるのか、そしてどの構造的要因が結果を支配しているのかを報告します。

> **ステータス: v0.1開発中。** 計画中の6コンポーネントのうち5つが実装済みです: `input_quality`/`applicability`、`ring_topology`、`size_topology`、`stereochemical_burden`、`functional_group_liability`。残るは`fragment_rarity`のみです。現在のスコープと未実装部分については[`docs/architecture.md`](docs/architecture.md)を参照してください。

## RENSEIがすること

* 分子を(`chematic`経由で)パースし、単一の数値ではなく構造化された`SynthesizabilityReport`を返す。
* 評価を独立したコンポーネントに分解する(現時点ではring topology、size/topology、stereochemical burden、functional-group liability、input quality/applicability。fragment rarityは今後実装予定)。
* **score**(合成容易性/困難性)、**confidence**(その判断の信頼性)、**applicability**(その分子がそもそもモデルの適用範囲内かどうか)を別々のフィールドとして分離する — 作りにくい分子だからといって自動的に低confidenceになるわけではない。
* 単なる文章ではなく、機械可読なfinding codeと構造化されたevidenceを出力する。
* retrosynthesis探索を一切実行しない。RENSEIは分子単体を評価するのみで、それを作るための経路を計画することはしない。
* 単一分子およびバッチ(`.sdf`/SMILESファイル)解析用の`rensei` CLIを同梱する — 詳細は下記「コマンドラインインターフェース」を参照。

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

## コマンドラインインターフェース

```bash
rensei analyze "C1CC2CCC1C2" --format json
rensei analyze --input molecules.sdf --format jsonl --output reports.jsonl
```

* `rensei analyze "<SMILES>" [--format human|json|jsonl]` — 引数として渡した単一分子を解析する。
* `rensei analyze --input <file> [--format human|json|jsonl] [--output <file>]` — バッチモード。`<file>`は`.sdf`ファイル、またはSMILES-per-lineファイル(空白区切りの名前列を任意で含む、標準的な`.smi`形式)のいずれか。
* バッチモードは入力順序を維持し、1レコードの失敗で全体を停止しない — 失敗したレコードはスキップされるのではなくエラーエントリになる(JSONの`"error"`フィールド、またはhuman形式の`ERROR:`ブロック)。プロセスの終了コードは、全レコードの処理が完了した後、1件でも失敗があった場合にのみ非ゼロになる。
* `jsonl`形式は単一分子モードとバッチモードの両方で同じ`{"input", "report"|"error"}`ラッパー形式を使う — どちらの呼び出し形式で生成しても、下流のline-by-lineパーサーは単一のスキーマを見ることになる。
* 終了コード: `0`成功、`1`分子のパース/解析失敗(単一分子モード)またはバッチ内の1件以上の失敗、`2`使用方法エラー(引数不正)。
* CLIが出力するレポートも、他のレポートと同じく`fragment_rarity: null`のギャップを持つ — 下記「制限事項」を参照。

## レポートの形式

`cargo run --example basic`の実際の出力(このコンポーネント構成時点のもの)。ノルボルナン(`C1CC2CCC1C2`)はring topologyのみを動かします:

```text
Verdict: ModeratelyAccessible
Synthesizability: 0.66
Confidence: 1.00
Dominant penalties:
1. Bridged ring system spanning 7 atoms — bridgehead connectivity typically increases synthetic difficulty.
```

立体中心が密集したフラグメント(`CC(O)C(N)C(C)C(O)C(N)C`)は`stereochemical_burden`と`functional_group_liability`(difficulty)、そして未指定の立体化学に対するapplicabilityの独立したconfidenceペナルティを動かします — difficultyとconfidenceが別々に動く点に注目してください:

```text
Verdict: ModeratelyAccessible
Synthesizability: 0.74
Confidence: 0.85
Dominant penalties:
1. 4 tetrahedral stereocenter(s) (specified or unspecified) requiring synthetic control.
2. Stereocenter density 0.33 is above the 0.25 threshold — stereocenters are concentrated in a compact region, leaving little room for staged, orthogonal control.
3. Reactive/unstable functional group detected: primary amine (Brenk et al. 2008 structural alert).
```

エポキシド(`C1CO1`)は`functional_group_liability`単体を動かします — chematicのBrenk et al.(2008)構造アラートセットを直接ラップしています:

```text
Verdict: LikelyAccessible
Synthesizability: 0.87
Confidence: 1.00
Dominant penalties:
1. Reactive/unstable functional group detected: epoxide (Brenk et al. 2008 structural alert).
```

fragment rarityが未実装のため、全体的なスコアは完全なv0.1が出す値より低めになります。

各レポートには`Provenance`ブロック(schema version、rensei version、chematic version、ruleset version、config hash)も含まれており、バージョン間で結果を比較できるようになっています — `docs/architecture.md`を参照してください。

## コンポーネントの実装状況(v0.1)

| コンポーネント | 状態 |
|---|---|
| `input_quality` / applicability | 実装済み |
| `ring_topology` | 実装済み |
| `size_topology` | 実装済み |
| `stereochemical_burden` | 実装済み(四面体型立体中心のみ — 「制限事項」参照) |
| `functional_group_liability` | 実装済み(反応性/不安定な官能基のみ — 「制限事項」参照) |
| `fragment_rarity` | 未実装 |

未実装のコンポーネントは`ComponentScores`内で`None`として表現され、捏造されたゼロスコアとしては表現されません。

## 既存ツールとの違い

* **SAscore**はフラグメント頻度と複雑性ペナルティを単一の数値として返します。RENSEIはコンポーネントごとの診断、confidence、applicability、evidence、(将来的には)改善提案を返します。
* **SYBA**はeasy/hardの二値分類器です。RENSEIは診断と説明を主眼としたツールです。
* **SCScore**は学習済みの合成複雑性スコアです。RENSEIは代わりに透明で化学的に名前の付いた要因へ分解します。
* **RAscore**はretrosynthesisの成功確率を近似します。RENSEIはroute-freeであり、その評価の構造的理由を説明します。
* **AiZynthFinder、ASKCOS、RENKIN**はroute plannerです。RENSEIは経路を生成しません。

## 制限事項

* v0.1では計画されている6コンポーネントのうち5つを実装しています(上の表を参照)。`overall.difficulty`/`overall.synthesizability`は現状ring topology、size/topology、stereochemical burden、functional-group liabilityのみを反映しています。
* `size_topology`のrotatable bond(回転可能結合)に関する項は、市販されている単純な非分岐長鎖分子(回転可能結合は多いが合成難易度はほぼゼロ)を過大評価します — これは既知のギャップであり、fragment rarity(未実装)がそうしたフラグメントを一般的/前例のあるものと認識することで補正される予定です。`docs/architecture.md`の "Scoring direction" セクションを参照してください。
* `stereochemical_burden`は四面体型立体中心の個数と密度のみを対象としています。E/Z二重結合の立体化学、atropisomerism、連続する立体中心、四級炭素への隣接、meso化合物の検出は未実装です — E/Zについては特に、chematicのE/Z判定に2D座標が必要であり、SMILESのみのパイプラインではそれを持っていないためです。
* `functional_group_liability`は反応性/不安定な官能基のみを対象とし、chematicのBrenk et al.(2008)構造アラートセットを直接ラップしています。相互に非互換な官能基の組み合わせ、密な官能基化、保護基の圧力、化学選択性の負担、多官能性の対称性破れ、難しい酸化状態の組み合わせは未実装です — 最後の項目はchematicに酸化状態関連のAPIが一切存在しないためです。Brenkのセットは医薬品化学のスクリーニングライブラリにおける「望ましさ」フィルターとして検証されたものであり、合成難易度のシグナルではありません。そのため一部のアラートは一般的で安価に前例のある官能基にも反応します — 例えばアスピリンは4つのBrenkアラートに引っかかり、合成が非常に容易であるにもかかわらず`ModeratelyAccessible`と判定されます。これは上記のrotatable bondの問題と同じ形の既知のギャップであり、同じ解決策(fragment rarityの実装)が見込まれています。
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

残りの計画: fragment rarity、SAscore/RAscore/route-search outcomeに対するキャリブレーション、そして将来的にはPythonバインディング。
