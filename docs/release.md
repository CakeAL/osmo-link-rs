# 移动端发布

推送新版本标签后，[发布工作流](../.github/workflows/release.yml) 会分别构建 Android 和 iOS
的 Release 版本。两端成功后才创建 GitHub Release，并上传签名的 arm64 APK 和未签名的
arm64 IPA。版本号由标签提供。GitHub 仍会自动附带源码归档。

## Android 签名

Android 首次发布前需要创建一把长期使用的发布密钥。如果之前已签名分发过 APK，沿用原来的密钥，
以便用户覆盖安装。把密钥库和密码保存在仓库外，并备份。

```sh
keytool -genkeypair -storetype JKS -keystore "$HOME/osmo-link-release.jks" \
  -alias osmo-link -keyalg RSA -keysize 2048 -validity 10000
openssl base64 -A -in "$HOME/osmo-link-release.jks" | pbcopy
```

在 GitHub 仓库的 **Settings → Secrets and variables → Actions** 添加以下 Repository secrets。
Base64 只是编码，把剪贴板内容直接粘贴到 Secret 输入框。

| Secret | 值 |
| --- | --- |
| `ANDROID_KEYSTORE_BASE64` | 密钥库文件的 Base64 内容 |
| `ANDROID_KEYSTORE_PASSWORD` | 密钥库密码 |
| `ANDROID_KEY_ALIAS` | 上例中的 `osmo-link` |
| `ANDROID_KEY_PASSWORD` | 该别名下的私钥密码，若生成时采用密钥库密码则填同一值 |

## 标签和更新日志

发布前在 [CHANGELOG.md](../CHANGELOG.md) 顶部新增与标签一致的二级标题，如 `## v0.1.0`，
并写入更新内容。GitHub Release 正文取自该章节。从包含这份更新日志的提交打标签并推送。
小版本号和补丁号最多为 999，主版本号最多为 2099，各段省略前导零。同一版本选用一种标签写法。

```sh
git tag -a v0.1.0 -m "发布 v0.1.0"
git push origin v0.1.0
```

## iOS 签名

GitHub Actions 发布未签名的 IPA。用户下载后可以用自己的免费 Apple 账号为自己的设备
签名和续签。使用 Xcode Personal Team 也能安装到个人设备，开发描述文件有效期为 7 天。
[侧载步骤](ios.md#侧载) · [Apple Personal Team](https://developer.apple.com/help/account/basics/about-your-developer-account)

构建时预先签名需要签名证书、对应的私钥，以及包含目标设备的有效描述文件。免费账号适合
个人设备；用仓库作者的免费账号签出的 IPA 仍受设备和有效期约束。Apple 的 Ad Hoc
分发需要付费计划、分发证书和已注册设备。
[Apple Ad Hoc 分发说明](https://developer.apple.com/help/account/provisioning-profiles/create-an-ad-hoc-provisioning-profile)
