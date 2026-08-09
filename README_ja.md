# YOMITOKI

[![CI](https://github.com/kent-tokyo/yomitoki/actions/workflows/ci.yml/badge.svg)](https://github.com/kent-tokyo/yomitoki/actions/workflows/ci.yml)

高速で説明可能な、経路探索を伴わない分子合成容易性診断ライブラリ。

YOMITOKIは、[chematic](https://github.com/kent-tokyo/chematic)上に構築された、高速・説明可能・route-free(経路探索を伴わない)な分子合成容易性診断ライブラリです。

単一の合成容易性スコアだけを返すのではなく、YOMITOKIは分子構造を読み解き、なぜその分子が作りやすい、あるいは作りにくいと評価されたのかを説明します。判断の根拠となる構造的な証拠を特定し、その判断がどの程度信頼できるのかを報告します。

名前は日本語の「読み解き」に由来します。対象を注意深く読み、その意味や背景を明らかにするという意味です — 分子を作り変えるのではなく、読み解いて説明することが、このライブラリの役割そのものです。

> YOMITOKIは合成容易性を推定するだけではなく、その推定の根拠と推論過程を明らかにします。

> **旧名称はRENSEIです。** プロジェクトの実際の役割により合致するよう改名しました。その名称ではcrates.ioに公開されたことがないため、これは非推奨エイリアスではなく、クリーンな改名です。

> **ステータス: v0.1開発中。** 計画中の6コンポーネントのうち5つが実装済みです: `input_quality`/`applicability`、`ring_topology`、`size_topology`、`stereochemical_burden`、`functional_group_liability`。残るは`fragment_rarity`のみです。現在のスコープと未実装部分については[`docs/architecture.md`](docs/architecture.md)を参照してください。

## 位置付け

```text
chematic    分子表現とケモインフォマティクス
    |
YOMITOKI    分子の合成容易性を読み解き、説明する
    |
renkin      逆合成ルートを計画する
```

YOMITOKIはroute searchを一切実行しません — これはv0.1のスコープ上の制約ではなく、恒久的な境界です。詳細は下記「YOMITOKIがしないこと」を参照してください。

## YOMITOKIがすること

* 分子を(`chematic`経由で)パースし、単一の数値ではなく構造化された`SynthesizabilityReport`を返す。
* 評価を独立したコンポーネントに分解する(現時点ではring topology、size/topology、stereochemical burden、functional-group liability、input quality/applicability。fragment rarityは今後実装予定)。
* **score**(合成容易性/困難性)、**confidence**(その判断の信頼性)、**applicability**(その分子がそもそもモデルの適用範囲内かどうか)を別々のフィールドとして分離する — 作りにくい分子だからといって自動的に低confidenceになるわけではない。
* 単なる文章ではなく、機械可読なfinding codeと構造化されたevidenceを出力する。
* retrosynthesis探索を一切実行しない。YOMITOKIは分子単体を評価するのみで、それを作るための経路を計画することはしない。
* 単一分子およびバッチ(`.sdf`/SMILESファイル)解析用の`yomitoki` CLIを同梱する — 詳細は下記「コマンドラインインターフェース」を参照。
* `analyze_batch(&[Molecule], &AnalysisConfig) -> Vec<Result<...>>` — CLIやファイル形式を経由せず、ライブラリ呼び出し側にも同じ入力順序保証付きのバッチ処理エントリポイントを提供する。

## YOMITOKIがしないこと

* Retrosynthesisプランニング、reaction template適用、precursor生成、route ranking — これらは[RENKIN](https://github.com/kent-tokyo/renkin)の役割です。
* 分子パース、ring perception、aromaticity、stereochemistry割り当て — これらは[chematic](https://github.com/kent-tokyo/chematic)の役割であり、YOMITOKIはそれを利用するだけです。
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

* `yomitoki analyze "<SMILES>" [--format human|json|jsonl]` — 引数として渡した単一分子を解析する。
* `yomitoki analyze --input <file> [--format human|json|jsonl] [--output <file>]` — バッチモード。`<file>`は`.sdf`ファイル、またはSMILES-per-lineファイル(空白区切りの名前列を任意で含む、標準的な`.smi`形式)のいずれか。
* バッチモードは入力順序を維持し、1レコードの失敗で全体を停止しない — 失敗したレコードはスキップされるのではなくエラーエントリになる(JSONの`"error"`フィールド、またはhuman形式の`ERROR:`ブロック)。プロセスの終了コードは、全レコードの処理が完了した後、1件でも失敗があった場合にのみ非ゼロになる。
* `jsonl`形式は単一分子モードとバッチモードの両方で同じ`{"input", "report"|"error"}`ラッパー形式を使う — どちらの呼び出し形式で生成しても、下流のline-by-lineパーサーは単一のスキーマを見ることになる。
* 終了コード: `0`成功、`1`分子のパース/解析失敗(単一分子モード)またはバッチ内の1件以上の失敗、`2`使用方法エラー(引数不正)。
* CLIが出力するレポートも、他のレポートと同じく`fragment_rarity: null`のギャップを持つ — 下記「制限事項」を参照。

## レポートの形式

`cargo run --example basic`の実際の出力(このコンポーネント構成時点のもの)。ノルボルナン(`C1CC2CCC1C2`)はring topologyを動かし、その架橋環は簡略化提案も生成します:

```text
Verdict: ModeratelyAccessible
Synthesizability: 0.66
Confidence: 1.00
Dominant penalties:
1. Bridged ring system spanning 7 atoms — bridgehead connectivity typically increases synthetic difficulty.
Simplification suggestions (heuristic, not a guarantee):
1. ReplaceBridgedRingWithMonocyclicAnalog: Bridgehead connectivity in this ring system is a direct driver of the ring_topology contribution to difficulty. A monocyclic (or less-fused) analog, if the target application allows one, would remove this specific burden — this is a structural heuristic, not a guarantee the replacement is chemically equivalent or that synthesis actually becomes easier.
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

アラニナート(`C[C@@H](N)C(=O)[O-]`、脱プロトン化アラニン)は指定済み立体中心*と*負電荷原子の両方を持ちます — 後者はchematicの既知のバグです([#267](https://github.com/kent-tokyo/chematic/issues/267))。クラッシュしたり推測したりすることなく安全に立体解析をスキップし、「未指定立体中心」のケースとは明確に区別してconfidenceを下げます:

```text
Verdict: LikelyAccessible
Synthesizability: 0.92
Confidence: 0.60
Dominant penalties:
1. Reactive/unstable functional group detected: primary amine (Brenk et al. 2008 structural alert).
2. Reactive/unstable functional group detected: acetal ketal (Brenk et al. 2008 structural alert).
3. Stereo analysis could not be run for this molecule: it contains a negatively charged atom, which triggers an arithmetic-overflow bug in chematic's stereo perception (panics in debug builds, produces an unverified result in release builds — see chematic issue #267). Stereocenter count/density and stereo completeness are unavailable, not verified to be zero/complete.
```

fragment rarityが未実装のため、全体的なスコアは完全なv0.1が出す値より低めになります。

各レポートには`Provenance`ブロック(schema version、yomitoki version、chematic version、ruleset version、config hash)も含まれており、バージョン間で結果を比較できるようになっています — `docs/architecture.md`を参照してください。

## コンポーネントの実装状況(v0.1)

| コンポーネント | 状態 |
|---|---|
| `input_quality` / applicability | 実装済み |
| `ring_topology` | 実装済み |
| `size_topology` | 実装済み |
| `stereochemical_burden` | 実装済み(四面体型立体中心のみ — 「制限事項」参照) |
| `functional_group_liability` | 実装済み(反応性/不安定な官能基 + dense functionalization — 「制限事項」参照) |
| `fragment_rarity` | 未実装 |

未実装のコンポーネントは`ComponentScores`内で`None`として表現され、捏造されたゼロスコアとしては表現されません。

`suggestions: Vec<SimplificationSuggestion>`は6種類のコードのうち3種類(`ReplaceBridgedRingWithMonocyclicAnalog`、`SimplifyMacrocyclicClosure`、`ReduceStereocenterDensity`)について生成されます — 詳細は「制限事項」を参照。すべての提案は診断的・heuristicなものであり、確実性を主張することはありません(`expected_effect`は常に`MayReduceDifficulty`であり、`LikelyReducesDifficulty`にはなりません)。

## 既存ツールとの違い

* **SAscore**はフラグメント頻度と複雑性ペナルティを単一の数値として返します。YOMITOKIはコンポーネントごとの診断、confidence、applicability、evidence、簡略化提案を返します。
* **SYBA**はeasy/hardの二値分類器です。YOMITOKIは診断と説明を主眼としたツールです。
* **SCScore**は学習済みの合成複雑性スコアです。YOMITOKIは代わりに透明で化学的に名前の付いた要因へ分解します。
* **RAscore**はretrosynthesisの成功確率を近似します。YOMITOKIはroute-freeであり、その評価の構造的理由を説明します。
* **AiZynthFinder、ASKCOS、RENKIN**はroute plannerです。YOMITOKIは経路を生成しません。

## SAscoreとの比較

`chematic::chem::sa_score`(Ertl & Schuffenhauer 2009)に対する最小限のin-process比較です — AGENTS.md §27の完成条件の一つです。キャリブレーションや精度の主張ではありません: 両者は互いにフィッティングされておらず、測定しているものも異なります(SAscore: フラグメント頻度 + 複雑性ペナルティ、YOMITOKI: コンポーネントごとに分解された構造的負担)。スケールも逆方向です — SAscoreは`1`(易しい)〜`10`(難しい)、YOMITOKIの`difficulty`は`0.0`〜`1.0`で、ここでは共通軸への再スケーリングは行っていません。

`cargo run --example sa_score_comparison`の実際の出力:

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

興味深いのは両者が一致する行ではなく、乖離する行です — 乖離は自動的にYOMITOKIのバグを意味しません。最も顕著なのはアシルクロリドです: SAscoreは`6.62`(フラグメントが珍しいと判定)、YOMITOKIは`0.09`(`LikelyAccessible`)— 安価でありふれたアシル化試薬で、YOMITOKI自身のモデルでは構造的負担がほぼありません。アスピリン(`4.67` 対 `0.27`)は、既に「制限事項」で説明したBrenk妥当性のギャップと同じ形です。両者がおおむね一致する場合(カフェイン、スピロ環、架橋環+立体中心)も、どちらかが「正しい」ことの証拠にはなりません — どちらも実際の合成結果に対してまだ検証されていません。

## 制限事項

* v0.1では計画されている6コンポーネントのうち5つを実装しています(上の表を参照)。`overall.difficulty`/`overall.synthesizability`は現状ring topology、size/topology、stereochemical burden、functional-group liabilityのみを反映しています。
* 負電荷原子(カルボン酸イオン、スルホン酸イオン、リン酸イオンなど)を含む分子では、立体解析(`stereo_complete`と`stereochemical_burden`全体)が一切実行できません — これはchematicの実際のバグであり([#267](https://github.com/kent-tokyo/chematic/issues/267))、設計上の選択ではありません。YOMITOKIはこれに対してクラッシュしたり推測したりすることは一切ありません(`ApplicabilityReport.stereo_uncheckable`と`StereoAnalysisSkipped`フィンディングを参照)が、上流で修正されるまではそのような分子について立体化学のシグナルを一切持ちません。
* `size_topology`のrotatable bond(回転可能結合)に関する項は、市販されている単純な非分岐長鎖分子(回転可能結合は多いが合成難易度はほぼゼロ)を過大評価します — これは既知のギャップであり、fragment rarity(未実装)がそうしたフラグメントを一般的/前例のあるものと認識することで補正される予定です。`docs/architecture.md`の "Scoring direction" セクションを参照してください。
* `stereochemical_burden`は四面体型立体中心の個数と密度のみを対象としています。以下は調査した上でなお未実装です。理由はそれぞれ異なります(詳しい根拠は`docs/architecture.md`参照):
  * E/Z二重結合の立体化学 — chematicはSMILESの`/`/`\`結合マーカーから直接E/Zを割り当てられます(2D座標は不要 — ここに以前あった記述は誤りでした)。ただし入力SMILESが実際にマークした結合にしか適用されません。四面体型中心にある`stereo_completeness`のような「立体化学的だが未指定」の二重結合を検出する仕組みが存在しないため、指定済みのものだけをカウントすると「SMILESがどれだけ丁寧に書かれたか」を測ることになってしまいます — これは下記のatropisomerismを却下した理由と同じ種類の問題が、別の形で現れたものです。
  * Atropisomerism — chematicの`detect_atropisomers`を実際にテストし、不採用としました。同一分子でも`c1ccccc1-c2ccccc2`と書くとatropisomer判定され、`c1ccccc1c2ccccc2`と書くと判定されず、さらに*para*置換ビフェニルを本物のヒンダードな*ortho*置換ビフェニルと同一に扱います。これをラップするとYOMITOKI自身のatom順序/表記に対する不変性の保証に反します。
  * 連続する立体中心、四級炭素への隣接 — どちらも(指定済み・未指定を問わない)原子レベルの立体中心候補リストが必要ですが、chematicは集計カウントしか公開していません。これをYOMITOKI内で自前実装すると、検証済みのchematic primitiveを利用する側から、立体中心認識そのものを所有する側へと踏み出すことになります — これまで実装したすべてのコンポーネントが越えていない一線です。
  * meso化合物検出 — グラフ自己同型/位相的対称類が必要です。chematicは内部的にこれを持っています(`chematic-smiles::canonical_automorphism`)が、外部には公開していません。
* `functional_group_liability`は反応性/不安定な官能基(chematicのBrenk et al. 2008構造アラートセットを直接ラップ)と、dense functionalization(chematicのErtl 2017 `identify_functional_groups`による、互いに独立した官能基クラスターの個数)をカバーしています。相互に非互換な官能基の組み合わせと保護基の圧力は未実装です — 上記2つと異なり、どちらも根拠となる検証済みのprimitiveがchematicに存在せず、どちらかを手作業でキュレーションすることはAGENTS.mdが警告する「化学的に弱いルールを過剰に一般化する」ことそのものになってしまうためです。化学選択性の負担、多官能性の対称性破れ、難しい酸化状態の組み合わせも未実装です — 最後の項目はchematicに酸化状態関連のAPIが一切存在しないためです。Brenkのセットは医薬品化学のスクリーニングライブラリにおける「望ましさ」フィルターとして検証されたものであり、合成難易度のシグナルではありません。そのため一部のアラートは一般的で安価に前例のある官能基にも反応します — 例えばアスピリンは4つのBrenkアラートに引っかかり、合成が非常に容易であるにもかかわらず`ModeratelyAccessible`と判定されます。これは上記のrotatable bondの問題と同じ形の既知のギャップであり、同じ解決策(fragment rarityの実装)が見込まれています。dense functionalizationにも独自の既知のギャップがあります:位相的に「分離している」官能基クラスターの個数を数えるため、密に相互接続した多官能性システム(グルコースの水酸基が連なる環や、縮環したβ-ラクタムなど)は1つのクラスターに収束してしまい、官能基が1つしかない分子と同じカウントになります。
* fragment-rarityのコーパスがまだ存在しないため、新規/希少な部分構造は検出されません。
* 簡略化提案は`SuggestionCode`の6種類のうち3種類(架橋環、macrocycle、立体中心密度)のみをカバーしています。残る3種類には、まだ存在しない信号が必要です:四級炭素の隣接はどこでも計算されておらず、`brenk_matches_detailed`はパターンごとに原子をまとめて返す(occurrence単位ではない)ため「複数ある類似の反応性基のうち1つを除去する」提案がどの出現を指すべきか特定できず、「fragment precedentを増やす」にはfragment rarityが必要ですが後回しにされています。すべての提案のconfidenceは提案コードごとではなく一律の固定値(0.5)です — 実際の合成結果によるキャリブレーションが存在しないためです。
* `ApplicabilityReport.domain_distance`は、キャリブレーション用コーパスが存在するまで(Phase 2以降)常に`None`です。
* 対応範囲は厳選されたorganic元素のサブセットに限定されており、全元素対応やorganometallicへの対応は試みていません。
* スコアと閾値はルールベースであり、これまで外部ベンチマークに対する検証は行われていません。キャリブレーションや比較結果はまだ存在しません。

## 再現性

同一の入力、同一の`AnalysisConfig`、同一のyomitoki/chematic/rulesetバージョンであれば、`analyze`/`analyze_smiles`は常に同じレポートを返します — コアの評価処理には乱数を一切使用していません。

## ライセンス

[Apache License, Version 2.0](LICENSE-APACHE)または[MITライセンス](LICENSE-MIT)のいずれかの下でライセンスされています(選択可能)。

## 引用

現時点で論文や引用可能なリリースは存在しません。

## ロードマップ

残りの計画: fragment rarity、SAscore/RAscore/route-search outcomeに対する*キャリブレーション*(SAscoreに対する最小限の*比較*はキャリブレーションではなく、既に上記の通り実装済みです)、そして将来的にはPythonバインディング。
