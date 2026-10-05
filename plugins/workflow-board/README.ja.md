# rustdsh用workflow memberボード

DSHの読み取り専用進捗ボードを、固定したsource patchと実行可能な適用・診断ツールとして同梱します。既存の独立したplugin導入ツールと同じ配置を使い、launcherや実行エンジンは変更しません。`workflow-run`を二重登録するpluginではありません。

patchはDSHのUI・テスト・文書・lockfileの18ファイルだけを変更します。baseは `f97c0438fb1608bbc4c08c88a27344249795ea22`（source package `0.1.7-rc.2`）、board commitは `7bd9ac31c23fe369d1fa9a6849869f04967d6ee5` です。`manifest.json`にchecksum・commit・変更ファイルを固定し、無関係なDSH sourceや履歴はvendorしません。DSHのMIT noticeは `DSH-LICENSE` に保持し、ZCode sourcecopyはありません。

## 対応sourceの準備

Node.js 24とGitが必要です。検証済みbaseのcleanな隔離DSH checkoutを選択します。例えば `sahenjp/deepseek-harness` を別ディレクトリへcloneし、上記baseでdetached checkoutします。global npm package、導入済みprofile、他タスクのcheckoutは指定しないでください。

rustdsh checkoutから実行します。PowerShellでも同じコマンドです。

```sh
node plugins/workflow-board/source-patch.mjs --source ../dsh-board-source --check
node plugins/workflow-board/source-patch.mjs --source ../dsh-board-source --apply
```

既定は書き込まないcheckです。applyはsource root・exact base・patch checksum・18ファイルの一致とcleanな作業ツリーを要求し、Gitでpatch全体を事前確認してから適用し、reverse checkで検証します。成功時は `applied`、繰返し時は `already-applied` を返します。異なるrevision、dirty checkout、競合、改変patch、導入済みpackageディレクトリはJSON診断codeで拒否し、事前確認に失敗した既存ファイルは保持します。

適用したDSH sourceは通常のworkspace手順でbuildし、そのbuildの導入方法を明示的に選択してください。このツールはpackage導入・build script実行・DSH差替え・profile変更・タスク開始/停止を行いません。rdshは選択した元DSHへ委譲を続けます。上流更新や異なるsource revisionには再検証したpatchが必要で、互換性を推測して適用しません。

## 表示内容と検証

既存の永続 `workflow-run` renderer/reducerを拡張し、会話内toggleと既存右ペインのtabに、開始済みmemberだけをrunId＋member.seqで表示します。running・completed・failed・cancelled・interrupted、memberラベル、厳密なphase、成功完了数/開始数を表示し、model名・未開始queue・課題数・結果・成果物は捏造しません。子会話リンクは親のdirect-child catalogとrunning条件を維持します。SessionごとにChat keyだけを保存し、live projectionから復元します。狭い画面、折畳み、Escape、focus保持に対応します。

```sh
node --test plugins/workflow-board/tests/source-patch.test.mjs
```

適用ツールは一時Git fixtureでcheck、実適用、繰返し、空白path、dirty/untracked、root/revision違い、checksum/対象違い、複数ファイル競合時の非部分適用、導入済みpackage拒否を確認します。network・model・profileは不要で、CIはLinux/macOS/Windowsで実行します。

同梱board sourceはDSHのfocused 49テストと、実Chromeの1440px/390px・明暗テーマ・reload・Session切替・live状態移動・focus・Escape・正規リンク・text escapeのfixture QAを通過しています。productionコンポーネント＋scriptedイベントの検証で、導入済み実サーバーの確認ではありません。実導入/差替え、実workflowと完全なDSH build/GUI/web gateは未検証です。以前の導入済みDSH `0.2.0-rc.2`は関連イベントの公開接点が一致しますが、このsource baseではなく、自動patchしません。関連：[rustdsh #83](https://github.com/sahenjp/rustdsh/issues/83)。
