## インストール

**`beautiful-wallpaper_<version>_x64-setup.exe`** をダウンロードして実行してください。
`.msi` は同じ中身の別形式で、そちらを好む人やポリシーで配布する人向けです。

現在のユーザーにのみインストールするため、管理者権限は求められません。

## Windows が警告を出します。それは正しい挙動です

**このインストーラーには署名がありません。**そのため SmartScreen が「Windows に
よって PC が保護されました」と表示し、実行ボタンを「詳細情報」の下に隠します。
この警告はこのシェル固有のものではなく、発行元を確認できないインストーラー全般
に対して Windows が言うことで、既定の挙動として正しいものです。

署名にはコード署名証明書が必要で、購入と身元確認が伴います。このプロジェクトは
それを持っていません。未署名のインストーラーを実行したくない場合は、
[自分でビルド](https://github.com/C0uki/beautiful-wallpaper/blob/main/README.ja.md#インストール)
してください — `pnpm install` の後 `pnpm --filter @bw/shell app:build` で、
読めるソースから同じバンドルが作れます。

## 必要なもの

- **Windows 10 または 11。**
- **[WebView2](https://developer.microsoft.com/microsoft-edge/webview2/)。**
  Windows 11 には標準で入っており、Windows 10 でも大抵は入っています。
  インストーラーには同梱していません。

## 初回起動

初回起動の画面で、壁紙・バー・Windows 統合・キー割り当て — Windows が明け渡さ
なかったキーの組み合わせと、代わりに使える手段も含めて — を一通り案内します。
`bw wizard open` で後からもう一度開けます。

## 何かがうまく動かないとき

多くはバグではなく、Windows が何かを渡してくれていないだけです。空のトレイ、
動かない輝度調整、タイリングウィンドウマネージャーが無いときのワークスペースの
欠如など。[docs/troubleshooting.md](https://github.com/C0uki/beautiful-wallpaper/blob/main/docs/troubleshooting.md)
に、詰みかねない例外的な 1 件 — シェルが残っていない状態でタスクバーが隠れて
しまうケースを、タスクマネージャーの **新しいタスクの実行** から `bw taskbar show`
で戻す方法込みで — まとめてあります。

設定項目はすべて [docs/config.md](https://github.com/C0uki/beautiful-wallpaper/blob/main/docs/config.md)
にあります。
