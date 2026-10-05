# rdsh-settings (test prototype)

DSH設定サイドバーに rdsh セクションを追加するテスト用プラグインです。
デザインはDSHそのまま (settings.section スロットを使い、DSH変数で描画)。

## 入る場所

- 設定モーダルの縦ナビに rdsh が増えます
- 順序は Models(10) / Built-in plugins(15) の次: order 20
- クリックでトークン予算・retriever/packer/verifier・goal・作業ファイル・未解決タスクを調整できます

## 保存先

- DSH_HOME/rdsh-context.json (なければ HOME/.dsh/rdsh-context.json)
- Rust側 `rdsh context build/search/status/explain` と同じファイルを読み書きします

## 試す

```sh
dsh plugin --profile web add ./plugins/rdsh-settings
```

外すときはプロファイルのプラグイン一覧から rdsh-settings を外します。
テスト実装なので既定は全てON・budget 4000です。
