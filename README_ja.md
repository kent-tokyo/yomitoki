# yomitoki

[![CI](https://github.com/kent-tokyo/yomitoki/actions/workflows/ci.yml/badge.svg)](https://github.com/kent-tokyo/yomitoki/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/yomitoki.svg)](https://crates.io/crates/yomitoki)
[![docs.rs](https://docs.rs/yomitoki/badge.svg)](https://docs.rs/yomitoki)
[![License](https://img.shields.io/crates/l/yomitoki.svg)](#ライセンス)

[English](README.md) | **日本語** | [中文](README_zh.md)

高速で説明可能な、経路探索を伴わない分子合成容易性診断ライブラリ。

yomitokiが診断するのは**intrinsic structural synthesizability(分子固有の構造的合成容易性)**です — 対象分子そのものから説明できる負荷(サイズ、ring topology、stereochemistry、functional-group liability)です。**route-dependent difficulty(経路・文脈依存の困難性)** — precursorの入手可能性、route長、protecting-group戦略、retrosynthesis探索の成否 — は予測しません。これは現時点の制約ではなく設計上の境界です。それは別の、外部文脈に依存する問いであり、[RENKIN](https://github.com/kent-tokyo/renkin)の役割です。なぜこれが「scienceに基づく測定対象の明確化」であって「scopeの後退」ではないのかは、下記「位置付け」を参照してください。

yomitokiは、[chematic](https://github.com/kent-tokyo/chematic)上に構築された、高速・説明可能・route-free(経路探索を伴わない)な分子合成容易性診断ライブラリです。

単一の合成容易性スコアだけを返すのではなく、yomitokiは分子構造を読み解き、なぜその分子が作りやすい、あるいは作りにくいと評価されたのかを説明します。判断の根拠となる構造的な証拠を特定し、その判断がどの程度信頼できるのかを報告します。

名前は日本語の「読み解き」に由来します。対象を注意深く読み、その意味や背景を明らかにするという意味です — 分子を作り変えるのではなく、読み解いて説明することが、このライブラリの役割そのものです。

> yomitokiは合成容易性を推定するだけではなく、その推定の根拠と推論過程を明らかにします。

> **ステータス: `0.2.0-alpha.1` — `0.1.1`をベースにした`v0.2` accuracy-redesign世代の最初のリリース。** これまでに本物のscoring behavior変更が2つ入っています: `ring_topology`のring-family集約がplain sumからL2 normへ変更され、`size_topology`にheteroatom-count burden termが追加されました(weightは較正の上`0.03`でfreeze済み)。どちらもdevelopment set(MPScore)上の知見であり、TS1/TS2/TS3と新しいholdoutはまだ再評価していません — そのためまだalphaです。各変更の全根拠は[`CHANGELOG.md`](CHANGELOG.md)を参照してください。計画中の6コンポーネントすべてが実装済みです(`fragment_precedent`含む — `0.1.0-alpha.2`公開後のround 18で`fragment_rarity`から改名。難易度を上げる方向にも下げる方向にも作用するようになった以上、「rarity detector」という名前はもう実態に合っていませんでした)。opt-inです — yomitoki自体にはfragment-frequencyコーパスが同梱されていないため(AGENTS.md §5.4は巨大バイナリとしてコーパスを直接埋め込むことを禁止しています)、コーパスを構築(`tools/build-fragment-corpus/`)して設定(`AnalysisConfig.fragment_model`)しない限り無効のままです。**`fragment_precedent`は説明用の参照コーパス信号であり、合成難易度そのものを表す項ではありません** — round 20で、この信号をscoringの入力として信頼するにはcorpus依存性が大きすぎることが判明しました(誠実にsynthesis-focusedとラベル付けされた2つのコーパス同士が、500 probe分子のうち34.6%で前例信号の方向について食い違い、化学的に何の問題もないピリジン単体が、どのコーパスを設定するかだけで`LikelyAccessible`と`HighlyChallenging`の間を反転しました)。そのためround 21で`overall.difficulty`から完全に切り離しました(option C)。**コーパスを設定した場合:** `fragment_precedent`は引き続き計算・報告されますが(`SynthesizabilityReport.fragment_precedent`)、`overall.difficulty`・`dominant_penalties`・`dominant_supports`を変化させることはできません — コーパスを設定しても(あるいは別のコーパスに切り替えても)スコアは変わらず、付随するevidenceだけが変わります。詳細な前後比較は「制限事項」、round 16〜21の全経緯は`rules.rs`の"Fragment precedent"セクションを参照してください。詳細は[`CHANGELOG.md`](CHANGELOG.md)を参照してください。現在のスコープと未実装部分については[`docs/architecture.md`](docs/architecture.md)を参照してください。

## 位置付け

```text
chematic
  ↓ 分子・反応のprimitive
yomitoki
  ↓ intrinsic structural synthesizability(分子固有の構造的合成容易性)
    「この分子の何が構造的に合成を難しくしているのか?」
renkin
     route-dependent planning and evidence(経路依存のplanning・evidence)
     「実際にどう作るか?」
```

[chematic](https://github.com/kent-tokyo/chematic) · [renkin](https://github.com/kent-tokyo/renkin)

yomitokiはroute searchを一切実行しません — これはv0.1のスコープ上の制約ではなく、恒久的な境界です。詳細は下記「yomitokiがしないこと」を参照してください。

**これは測定対象の明確化であり、scopeの後退ではありません。** yomitoki自身がv0.3で実際の特許文献ベースの合成route(PaRoutes)に対して行った評価により、intrinsic structural burden(分子固有の構造的負荷)とroute-dependent difficulty(経路依存の困難性)は、一つの量の二つの側面ではなく、本質的に別物であることが分かりました: 実際のroute長は、試したどのroute-freeな構造表現とも弱い相関しか持たず(最良でもρ≈0.23、本プロジェクト自身が事前登録した基準で言う「moderate」水準にすら届きません)、一方で既に購入可能な出発原料との類似度 — 単一分子の表現からは決して見えない情報 — の方が、特に構造的に複雑な対象分子ほど強く相関します。二つの軸を明示的に分けて呼ぶことは、evidenceが支持する結論であり、「intrinsic structural synthesizability」が何を測っているかを絞り込むものであって、範囲を狭めるものではありません。詳細な分析: [`benchmarks/synthesizability/v03_two_axis_product_framing/README.md`](benchmarks/synthesizability/v03_two_axis_product_framing/README.md)。yomitoki → RENKIN間のインターフェース契約自体(reportが何をどのような形でhand-offするか)はまだ正式化されていません — これは将来の作業であり、この位置付けが暗に示すものではありません。

## yomitokiがすること

* 分子を(`chematic`経由で)パースし、単一の数値ではなく構造化された`SynthesizabilityReport`を返す。
* 評価を独立したコンポーネントに分解する(ring topology、size/topology、stereochemical burden、functional-group liability、input quality/applicability、fragment precedent — 最後のものはopt-in、詳細は下記「制限事項」)。
* **score**(合成容易性/困難性)、**confidence**(その判断の信頼性)、**applicability**(その分子がそもそもモデルの適用範囲内かどうか)を別々のフィールドとして分離する — 作りにくい分子だからといって自動的に低confidenceになるわけではない。**`overall.difficulty`が意味するのはintrinsic structural difficulty(分子固有の構造的困難性)であり、予測されたroute difficultyではありません** — 実際の合成step数、precursorの入手可能性、route探索の成否を推定するものではありません。上記「位置付け」を参照。
* 単なる文章ではなく、機械可読なfinding codeと構造化されたevidenceを出力する。
* retrosynthesis探索を一切実行しない。yomitokiは分子単体を評価するのみで、それを作るための経路を計画することはしない。
* 単一分子およびバッチ(`.sdf`/SMILESファイル)解析用の`yomitoki` CLIを同梱する — 詳細は下記「コマンドラインインターフェース」を参照。
* `analyze_batch(&[Molecule], &AnalysisConfig) -> Vec<Result<...>>` — CLIやファイル形式を経由せず、ライブラリ呼び出し側にも同じ入力順序保証付きのバッチ処理エントリポイントを提供する。

## yomitokiがしないこと

* Retrosynthesisプランニング、reaction template適用、precursor生成、route ranking — これらは[RENKIN](https://github.com/kent-tokyo/renkin)の役割です。
* 分子パース、ring perception、aromaticity、stereochemistry割り当て — これらは[chematic](https://github.com/kent-tokyo/chematic)の役割であり、yomitokiはそれを利用するだけです。
* 毒性判定、SDS/危険物分類、収率予測、コスト予測。
* v0.1では全元素対応やorganometallic(有機金属)への完全対応は行いません。

## クイックスタート

```rust
use yomitoki::{analyze_smiles, AnalysisConfig};

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
yomitoki analyze "C1CC2CCC1C2" --format json
yomitoki analyze --input molecules.sdf --format jsonl --output reports.jsonl
```

* `yomitoki analyze "<SMILES>" [--format human|json|jsonl] [--fragment-corpus <dir>]` — 引数として渡した単一分子を解析する。
* `yomitoki analyze --input <file> [--format human|json|jsonl] [--output <file>] [--fragment-corpus <dir>]` — バッチモード。`<file>`は`.sdf`ファイル、またはSMILES-per-lineファイル(空白区切りの名前列を任意で含む、標準的な`.smi`形式)のいずれか。
* バッチモードは入力順序を維持し、1レコードの失敗で全体を停止しない — 失敗したレコードはスキップされるのではなくエラーエントリになる(JSONの`"error"`フィールド、またはhuman形式の`ERROR:`ブロック)。プロセスの終了コードは、全レコードの処理が完了した後、1件でも失敗があった場合にのみ非ゼロになる。
* `jsonl`形式は単一分子モードとバッチモードの両方で同じ`{"input", "report"|"error"}`ラッパー形式を使う — どちらの呼び出し形式で生成しても、下流のline-by-lineパーサーは単一のスキーマを見ることになる。
* 終了コード: `0`成功、`1`分子のパース/解析失敗(単一分子モード)またはバッチ内の1件以上の失敗、`2`使用方法エラー(引数不正)。
* `--fragment-corpus <dir>`は`tools/build-fragment-corpus`の出力ディレクトリを読み込み、その実行で`fragment_precedent`を有効化します(分子解析が始まる前に一度だけ読み込まれます)。指定しない場合、レポートは`fragment_precedent: null`のままです — このフラグが追加される前と同じです。yomitoki自体にはコーパスが同梱されていないためで、詳細は下記「制限事項」を参照。

## レポートの形式

`cargo run --example basic`の実際の出力(このコンポーネント構成時点のもの)。ノルボルナン(`C1CC2CCC1C2`)はring topologyを動かし、その架橋環は簡略化提案も生成します:

```text
Verdict: ModeratelyAccessible
Synthesizability: 0.66
Confidence: 1.00
Dominant penalties:
1. Bridged ring system spanning 7 atoms — bridgehead connectivity typically increases structural synthetic difficulty.
Simplification suggestions (heuristic, not a guarantee):
1. ReplaceBridgedRingWithMonocyclicAnalog: Bridgehead connectivity in this ring system is a direct driver of the ring_topology contribution to difficulty. A monocyclic (or less-fused) analog, if the target application allows one, would remove this specific burden — this is a structural heuristic, not a guarantee the replacement is chemically equivalent or that synthesis actually becomes easier.
```

立体中心が密集したフラグメント(`CC(O)C(N)C(C)C(O)C(N)C`)は`stereochemical_burden`と`functional_group_liability`(difficulty)、そして未指定の立体化学に対するapplicabilityの独立したconfidenceペナルティを動かします — difficultyとconfidenceが別々に動く点に注目してください:

```text
Verdict: ModeratelyAccessible
Synthesizability: 0.69
Confidence: 0.85
Dominant penalties:
1. 5 tetrahedral stereocenter(s) (specified or unspecified) requiring synthetic control.
2. Stereocenter density 0.42 is above the 0.25 threshold — stereocenters are concentrated in a compact region, leaving little room for staged, orthogonal control.
3. Reactive/unstable functional group detected: primary amine (Brenk et al. 2008 structural alert).
Simplification suggestions (heuristic, not a guarantee):
1. ReduceStereocenterDensity: Stereocenters are concentrated in a compact region, leaving little room for staged, orthogonal stereocontrol. Reducing the number of stereocenters, or spreading them further apart in the structure, would lower this contribution to difficulty — this is a structural heuristic, not a guarantee.
```

エポキシド(`C1CO1`)は`functional_group_liability`単体を動かします — chematicのBrenk et al.(2008)構造アラートセットを直接ラップしています:

```text
Verdict: LikelyAccessible
Synthesizability: 0.87
Confidence: 1.00
Dominant penalties:
1. Reactive/unstable functional group detected: epoxide (Brenk et al. 2008 structural alert).
```

9員環(`C1CCCCCCCC1`)は`ring_topology`のmacrocycle分岐を動かし、独自の簡略化提案を生成します:

```text
Verdict: LikelyAccessible
Synthesizability: 0.75
Confidence: 1.00
Dominant penalties:
1. Macrocyclic ring of 9 atoms (at or above the 9-atom macrocycle threshold).
Simplification suggestions (heuristic, not a guarantee):
1. SimplifyMacrocyclicClosure: Macrocyclic ring closure is a direct driver of the ring_topology contribution to difficulty (large-ring closures often need high-dilution or specialized macrocyclization methods). A smaller ring or acyclic analog, if chemically acceptable, would remove this burden — this is a structural heuristic, not a guarantee.
```

ペンタエリスリトールテトラアセテート(`CC(=O)OCC(COC(C)=O)(COC(C)=O)COC(C)=O`)は互いに隣接しない4つのエステル環境を持ち、`functional_group_liability`の「dense functionalization」シグナル(`chematic::chem::identify_functional_groups`、Ertl 2017クラスタリング)をBrenkアラートに加えて動かします:

```text
Verdict: LikelyAccessible
Synthesizability: 0.81
Confidence: 1.00
Dominant penalties:
1. 4 distinct functional-group environments (Ertl 2017 clustering), above the 3 threshold — multiple independent reactive/functional regions can compete for reagent selectivity and complicate protecting-group strategy.
2. Reactive/unstable functional group detected: ketone alpha (Brenk et al. 2008 structural alert).
3. Reactive/unstable functional group detected: acetal ketal (Brenk et al. 2008 structural alert).
```

アラニナート(`C[C@@H](N)C(=O)[O-]`、脱プロトン化アラニン)は指定済み立体中心*と*負電荷原子の両方を持ちます。chematic 0.12までは、負電荷がchematic側の実際のオーバーフローバグ([chematic#267](https://github.com/kent-tokyo/chematic/issues/267))を引き起こしていたため、yomitokiは負電荷原子を持つすべての分子で立体解析全体をスキップする回避策を取っていました(confidenceの低下と`stereochemical_burden`のゼロ固定という代償付きで)。**chematic 0.13.0でこのバグが上流で直接修正されたことを確認し**(アラニナートが中性の酸型と全く同じ結果を返すことを検証済み)、この回避策は削除されました。現在はこの分子も他の分子と同様に、完全かつ正しい立体解析を受けます:

```text
Verdict: LikelyAccessible
Synthesizability: 0.86
Confidence: 1.00
Dominant penalties:
1. 1 tetrahedral stereocenter(s) (specified or unspecified) requiring synthetic control.
2. Reactive/unstable functional group detected: primary amine (Brenk et al. 2008 structural alert).
3. Reactive/unstable functional group detected: acetal ketal (Brenk et al. 2008 structural alert).
```

fragment corpusを設定しても変わるのは`fragment_precedent`のevidenceの内容だけです — round 21(option C)以降、`overall.difficulty`/`overall.synthesizability`は`ring_topology`/`size_topology`/`stereochemical_burden`/`functional_group_liability`のみから計算され、コーパス設定の有無にかかわらず常に同一です。

各レポートには`Provenance`ブロック(schema version、yomitoki version、chematic version、ruleset version、fragment corpusのモデルバージョン、config hash)も含まれており、バージョン間で結果を比較できるようになっています — `docs/architecture.md`を参照してください。

## コンポーネントの実装状況(v0.1)

| コンポーネント | 状態 |
|---|---|
| `input_quality` / applicability | 実装済み |
| `ring_topology` | 実装済み |
| `size_topology` | 実装済み |
| `stereochemical_burden` | 実装済み(四面体型立体中心のみ — 「制限事項」参照) |
| `functional_group_liability` | 実装済み(反応性/不安定な官能基 + dense functionalization — 「制限事項」参照) |

`fragment_precedent`は実装済み・opt-in(`AnalysisConfig.fragment_model`にコーパスが設定されていない限り`None`)ですが、`ComponentScores`のフィールドでは**ありません** — round 21以降`overall.difficulty`に寄与しないため、独立したトップレベルの`SynthesizabilityReport.fragment_precedent: Option<FragmentPrecedentEvidence>`として報告されます(コーパス相対パーセンタイル信号、説明用evidence専用。「制限事項」参照)。

本当に評価されていないコンポーネント/`fragment_precedent`は`None`として表現され、捏造されたゼロスコアとしては表現されません。

`suggestions: Vec<SimplificationSuggestion>`は6種類のコードのうち3種類(`ReplaceBridgedRingWithMonocyclicAnalog`、`SimplifyMacrocyclicClosure`、`ReduceStereocenterDensity`)について生成されます — 詳細は「制限事項」を参照。`IncreaseFragmentPrecedent`はround 21で廃止されました(`overall.difficulty`に寄与しなくなった以上「これでdifficultyが下がる」という主張が成立しなくなったため、enum自体はschema安定性のため残しつつ発行しません)。すべての提案は診断的・heuristicなものであり、確実性を主張することはありません(`expected_effect`は常に`MayReduceDifficulty`であり、`LikelyReducesDifficulty`にはなりません)。

## 既存ツールとの違い

* **SAscore**はフラグメント頻度と複雑性ペナルティを単一の数値として返します。yomitokiはコンポーネントごとの診断、confidence、applicability、evidence、簡略化提案を返します。
* **SYBA**はeasy/hardの二値分類器です。yomitokiは診断と説明を主眼としたツールです。
* **SCScore**は学習済みの合成複雑性スコアです。yomitokiは代わりに透明で化学的に名前の付いた要因へ分解します。
* **RAscore**はretrosynthesisの成功確率を近似します。yomitokiはroute-freeであり、その評価の構造的理由を説明します。
* **BR-SAScore**はSAscoreのフラグメント表をUSPTOの反応データ/eMoleculesのbuilding block情報で再学習し、単一のreaction/building-block-informedスコアを返します。yomitokiは5つの独立した構造コンポーネントに加え、applicability・confidence・機械可読なfinding/簡略化提案コードを返します — 単一の数値ではなく構造化された診断レポートであり、`overall.difficulty`を算出するのに反応コーパスを一切必要としません(コーパスの選択によって`overall.difficulty`が変化しないことはcontractとして保証されています — 両者が精度でどう比較されるかは下記[外部ベンチマーク](#外部ベンチマークv010)を参照)。
* **AiZynthFinder、ASKCOS、RENKIN**はroute plannerです。yomitokiは経路を生成しません。

## SAscoreとの比較

`chematic::chem::sa_score`(Ertl & Schuffenhauer 2009)に対する最小限のin-process比較です — AGENTS.md §27の完成条件の一つです。キャリブレーションや精度の主張ではありません: 両者は互いにフィッティングされておらず、測定しているものも異なります(SAscore: フラグメント頻度 + 複雑性ペナルティ、yomitoki: コンポーネントごとに分解された構造的負担)。スケールも逆方向です — SAscoreは`1`(易しい)〜`10`(難しい)、yomitokiの`difficulty`は`0.0`〜`1.0`で、ここでは共通軸への再スケーリングは行っていません。

`cargo run --example sa_score_comparison`の実際の出力:

```text
molecule                                sa_score      yomitoki_diff  verdict
ethanol                                     3.45               0.01  LikelyAccessible
benzene                                     2.40               0.10  LikelyAccessible
norbornane (bridged)                        8.20               0.34  ModeratelyAccessible
stereocenter-dense fragment                 8.32               0.31  ModeratelyAccessible
epoxide                                     6.23               0.13  LikelyAccessible
aspirin                                     4.67               0.27  ModeratelyAccessible
paracetamol                                 4.56               0.24  LikelyAccessible
caffeine (fused heterocycle)                4.94               0.29  ModeratelyAccessible
acyl halide                                 6.62               0.09  LikelyAccessible
cyclopropane (strained)                     3.94               0.13  LikelyAccessible
nitrile                                     4.97               0.04  LikelyAccessible
alanine (specified stereocenter)            3.55               0.14  LikelyAccessible
bridged ring + several stereocenters       10.00               0.72  Challenging
spiro ring system                           5.52               0.20  LikelyAccessible
```

興味深いのは両者が一致する行ではなく、乖離する行です — 乖離は自動的にyomitokiのバグを意味しません。最も顕著なのはアシルクロリドです: SAscoreは`6.62`(フラグメントが珍しいと判定)、yomitokiは`0.09`(`LikelyAccessible`)— 安価でありふれたアシル化試薬で、yomitoki自身のモデルでは構造的負担がほぼありません。アスピリン(`4.67` 対 `0.27`)は、既に「制限事項」で説明したBrenk妥当性のギャップと同じ形です。両者がおおむね一致する場合(カフェイン、スピロ環、架橋環+立体中心)も、どちらかが「正しい」ことの証拠にはなりません — どちらも実際の合成結果に対してまだ検証されていません。

「stereocenter-dense fragment」と「bridged ring + several stereocenters」の`yomitoki_diff`はchematic 0.13.0へのアップグレードで変化しました(`0.27→0.31`、`0.69→0.72`)— 下記の負電荷原子の修正とは別の、立体中心の*カウント*に関するバグ修正です(暗黙水素のrank-0センチネルが、実原子の正規化rank 0と衝突し、特定の位置で立体中心を過小カウントしていました)。どちらの数値も正しい方向(増加)へ動いており、以前の値が過大評価だったのではなく過小評価だったということです。

## 外部ベンチマーク(v0.1.0)

yomitoki v0.1.0の凍結デフォルト設定を、BR-SAScore自身のTS1/TS2/TS3テストセット上で、SAscore・BR-SAScoreと全く同じ分子集合を用いて測定しました。完全な手法・分子ごとの結果・正直な限界事項は**[docs/benchmark.md](docs/benchmark.md)**を参照してください。

盛らずに率直に述べます: yomitokiはTS1でBR-SAScoreと拮抗しています(ROC-AUC 0.952 対 0.983。yomitoki自身の閾値におけるbalanced accuracyとMCCはSAscoreを上回ります)。**TS2では弁別力がゼロです**(ROC-AUC 0.476 — chanceレベル。これは真の構造的知見として診断済みです: TS2のeasy/hardクラスは、yomitokiのモデルが見るring/size/stereo/functional-groupの観点で均質です)。TS3では両競合よりも弱い結果でした(0.673 対 0.839 / 0.905)。このベンチマークが差別化要因として検証しようとしたconfidenceベースのselective prediction評価は**その仮説を裏付けませんでした** — TS1では、confidenceが高い予測の方が低い予測よりも明確に不正確でした。原因は`overall.confidence`が予測の正しさではなく、データセットの由来(stereoタグの完全性)の代理指標になっていたことにあります。これらはすべて`docs/benchmark.md`に、都合が良いからではなく事実だから報告されています — 同じ文書には、これらの数値が改善するために何が必要かも記されており、yomitokiのラウンド単位の開発プロセスはこの結果を再チューニングの材料としてではなく確認結果として扱います。

**TS2のchanceレベルという結果を、無関係な第二のデータセットで検証したところ、再現しました。** [`benchmarks/synthesizability/DEVELOPMENT_SET.md`](benchmarks/synthesizability/DEVELOPMENT_SET.md)では、凍結済みbaselineをMPScore(3人の専門化学者が独立にeasy/difficultを評価した公開データセットで、TS1/2/3のretrosynthesis plannerベースのラベルとは手法的に無関係、分子重複は約0.03%)に対して検証しています: 全体でROC-AUC 0.513、95%信頼区間はchance水準を含みます。アルゴリズムベースと人間ベースという2つの独立したground truthが、yomitokiの4つの構造コンポーネントが実際の合成困難性の多くを見落としていることで一致しています(このデータセットではfalse negativeが72.6%)。同文書ではラベルなしのablation panelも実施しており、どの構造軸にコンポーネントが応答するか(どこで飽和・逆転するか)を切り分け、4つのdesign-change候補をエビデンスベースで記録していますが、まだ何も実装していません — TS1/2/3のconfirmatory numbersとは意図的に分離したdevelopment専用の成果です。

## 制限事項

* 計画されている6コンポーネントすべてを実装しています(上の表を参照)が、`fragment_precedent`は**決して**`overall.difficulty`/`overall.synthesizability`に寄与しません — round 21(option C)以降、コーパスを設定(`AnalysisConfig.fragment_model`)しても`fragment_precedent`が報告する内容が変わるだけで、`overall.difficulty`が測るものは変わりません。`overall.difficulty`は常に`ring_topology`/`size_topology`/`stereochemical_burden`/`functional_group_liability`のみを反映します(コーパス設定の有無を問わず)。
* `size_topology`のrotatable bond(回転可能結合)に関する項は、市販されている単純な非分岐長鎖分子(回転可能結合は多いが合成難易度はほぼゼロ)を過大評価します — 例えばdodecaneの`overall.difficulty`は`0.068`、アスピリン(`functional_group_liability`で複数のBrenkアラートを引っかける)は`0.273`(`ModeratelyAccessible`)で、実際には最も合成が容易な分子の一つであるにもかかわらずこの評価になります。round 20までは、コーパスを設定すると`fragment_precedent`がこれを補正していました(dodecane `→ 0.000`、アスピリン `→ 0.095`)。**round 21でこの補正を削除しました**(理由は次の項目参照)。そのためこの過大評価は、コーパス設定の有無にかかわらず現在も未解決の既知の制限です。`fragment_precedent`は引き続き(スコア調整としてではなく説明用evidenceとして)そうしたフラグメントが強く前例づけられていることを報告するので、スコアには反映されなくてもレポート上でこの不一致を確認できます。
* **`fragment_precedent`は説明用の参照コーパス信号であり、合成難易度そのものを表す項ではありません** — これは単なる注意点ではなく公開contractです。round 17〜20では`overall.difficulty`に寄与していましたが、round 20のcross-corpus検証でそれが安全でないことが判明しました: ChEMBLと、2つの実在する誠実にsynthesis-focusedとラベル付けされたコーパス(Open Reaction Database、SynRXN)とを比較すると、構造的には妥当な分子(カフェイン、ノルボルナン、スピロ/立体中心密な系)の一部が、あるコーパスでは*より難しく*、別のコーパスではそうならず、事前にどちらになるか予測する方法がありませんでした — 最も極端な例では、化学的に何の問題もないピリジン単体(合成難易度の説明が成り立ちようがない分子)が、どのコーパスを設定するかだけで`LikelyAccessible`と`HighlyChallenging`の間を反転しました。これはこのコンポーネント単独の無制限なpenalty項だけによるものでした。**round 21(option C)で`fragment_precedent`を`overall.difficulty`から完全に切り離しました** — 信号自体は従来と全く同じ方法で計算・報告され続けますが(`SynthesizabilityReport.fragment_precedent`)、もはやスコア・verdict・`dominant_penalties`・`dominant_supports`のいずれも変化させることはできません。end-to-endで確認済み: `overall.difficulty`は、どのコーパス(ChEMBL/ORD/SynRXN/未設定)を設定してもテストしたすべての分子でビット単位で完全に一致します。round 16〜21の全経緯と理由は`rules.rs`の"Fragment precedent"セクションを参照してください。
* `stereochemical_burden`は四面体型立体中心の個数と密度のみを対象としています。以下は調査した上でなお未実装です。理由はそれぞれ異なります(詳しい根拠は`docs/architecture.md`参照):
  * E/Z二重結合の立体化学 — chematicはSMILESの`/`/`\`結合マーカーから直接E/Zを割り当てられます(2D座標は不要 — ここに以前あった記述は誤りでした)。ただし入力SMILESが実際にマークした結合にしか適用されません。四面体型中心にある`stereo_completeness`のような「立体化学的だが未指定」の二重結合を検出する仕組みが存在しないため、指定済みのものだけをカウントすると「SMILESがどれだけ丁寧に書かれたか」を測ることになってしまいます — これは下記のatropisomerismを却下した理由と同じ種類の問題が、別の形で現れたものです。
  * Atropisomerism — chematicの`detect_atropisomers`を実際にテストし、不採用としました。同一分子でも`c1ccccc1-c2ccccc2`と書くとatropisomer判定され、`c1ccccc1c2ccccc2`と書くと判定されず、さらに*para*置換ビフェニルを本物のヒンダードな*ortho*置換ビフェニルと同一に扱います。これをラップするとyomitoki自身のatom順序/表記に対する不変性の保証に反します。
  * 連続する立体中心、四級炭素への隣接 — どちらも(指定済み・未指定を問わない)原子レベルの立体中心候補リストが必要ですが、chematicは集計カウントしか公開していません。これをyomitoki内で自前実装すると、検証済みのchematic primitiveを利用する側から、立体中心認識そのものを所有する側へと踏み出すことになります — これまで実装したすべてのコンポーネントが越えていない一線です。
  * meso化合物検出 — グラフ自己同型/位相的対称類が必要です。chematicは内部的にこれを持っています(`chematic-smiles::canonical_automorphism`)が、外部には公開していません。
* `functional_group_liability`は反応性/不安定な官能基(chematicのBrenk et al. 2008構造アラートセットを直接ラップ)と、dense functionalization(chematicのErtl 2017 `identify_functional_groups`による、互いに独立した官能基クラスターの個数)をカバーしています。相互に非互換な官能基の組み合わせと保護基の圧力は未実装です — 上記2つと異なり、どちらも根拠となる検証済みのprimitiveがchematicに存在せず、どちらかを手作業でキュレーションすることはAGENTS.mdが警告する「化学的に弱いルールを過剰に一般化する」ことそのものになってしまうためです。化学選択性の負担、多官能性の対称性破れ、難しい酸化状態の組み合わせも未実装です — 最後の項目はchematicに酸化状態関連のAPIが一切存在しないためです。Brenkのセットは医薬品化学のスクリーニングライブラリにおける「望ましさ」フィルターとして検証されたものであり、合成難易度のシグナルではありません。そのため一部のアラートは一般的で安価に前例のある官能基にも反応します — 例えばアスピリンは4つのBrenkアラートに引っかかり、合成が非常に容易であるにもかかわらず`ModeratelyAccessible`と判定されます(round 21以降、コーパス設定の有無にかかわらず同じスコアです)。これは上記のrotatable bondの問題と同じ形の既知のギャップです — round 20まではコーパスを設定するとアスピリンの`overall.difficulty`は`0.273`から`0.095`へ補正されていましたが、round 21でその補正を削除しました(上のfragment_precedentのcontract説明を参照 — round 20でこの信号がscoringに使うには corpus依存性が大きすぎると判明したための判断であり、さらにチューニングを続ける代わりに補正機構自体を取り除きました)。dense functionalizationにも独自の既知のギャップがあります:位相的に「分離している」官能基クラスターの個数を数えるため、密に相互接続した多官能性システム(グルコースの水酸基が連なる環や、縮環したβ-ラクタムなど)は1つのクラスターに収束してしまい、官能基が1つしかない分子と同じカウントになります。
* yomitoki自体にはfragment corpusが同梱されていないため(AGENTS.md §5.4)、デフォルトでは新規/希少な部分構造は検出されません — `fragment_precedent`は、コーパスを構築(`tools/build-fragment-corpus`)して設定するまで`None`のままです。コーパスをデフォルトで同梱するかどうかはまだ決まっていません(AGENTS.md §5.4が示す`yomitoki-core`/`yomitoki-models`/`yomitoki-data`分割、またはfeature flag付き外部ファイル)。
* 簡略化提案は`SuggestionCode`の6種類のうち3種類(架橋環、macrocycle、立体中心密度)をカバーしています。残る3種類はそれぞれ別の理由で到達不能です:`IncreaseFragmentPrecedent`はround 21で廃止されました(`overall.difficulty`に寄与しなくなった以上「これでdifficultyが下がる」という主張が成立しないため)。四級炭素の隣接はどこでも計算されておらず、`brenk_matches_detailed`はパターンごとに原子をまとめて返す(occurrence単位ではない)ため「複数ある類似の反応性基のうち1つを除去する」提案がどの出現を指すべきか特定できません。すべての提案のconfidenceは提案コードごとではなく一律の固定値(0.5)です — 実際の合成結果によるキャリブレーションが存在しないためです。
* `ApplicabilityReport.domain_distance`は、キャリブレーション用コーパスが存在するまで(Phase 2以降)常に`None`です。
* 対応範囲は厳選されたorganic元素のサブセットに限定されており、全元素対応やorganometallicへの対応は試みていません。
* スコアと閾値はルールベースであり、ラベル付きデータセットに対してフィットさせたものではありません。外部ベンチマークは実施済みです(上記[外部ベンチマーク](#外部ベンチマークv010)を参照) — 結果は一様ではなく、TS1ではBR-SAScoreと拮抗、TS2ではchanceレベル、TS3では両競合より劣ります。この結果を受けてweight/thresholdを変更したことはまだありません(同セクションのtest-set-integrity方針を参照)。

## 再現性

同一の入力、同一の`AnalysisConfig`、同一のyomitoki/chematic/rulesetバージョンであれば、`analyze`/`analyze_smiles`は常に同じレポートを返します — コアの評価処理には乱数を一切使用していません。

## ライセンス

[Apache License, Version 2.0](LICENSE-APACHE)または[MITライセンス](LICENSE-MIT)のいずれかの下でライセンスされています(選択可能)。

## 引用

現時点で論文や引用可能なリリースは存在しません。

## ロードマップ

残りの計画: 同梱されるfragment corpus(`fragment_precedent`自体は実装済みですがopt-in — 「制限事項」参照)、SAscore/RAscore/route-search outcomeに対する*キャリブレーション*(SAscoreに対する最小限の*比較*はキャリブレーションではなく、既に上記の通り実装済みです)、そして将来的にはPythonバインディング。
