Vertex 使用文档

[English](./vertex.md) | 简体中文

**第一步：登录 GCP 控制台并导航至“API 和服务”**

1.  打开您的网络浏览器，访问 Google Cloud Console：`https://console.cloud.google.com/`
2.  使用您的 Google 账号登录。
3.  选择您希望为其创建 OAuth 2.0 凭据的 GCP 项目。如果您还没有项目，请先创建一个新项目。

**第二步：进入“凭据”页面**

1.  在控制台左侧的导航菜单中，找到并点击“API 和服务”(APIs & Services)。
2.  在展开的子菜单中，点击“凭据”(Credentials)。

**第三步：创建凭据**

1.  在“凭据”页面的顶部，点击“+ 创建凭据”(+ CREATE CREDENTIALS) 按钮。
2.  在弹出的下拉菜单中，选择“OAuth 客户端 ID”(OAuth client ID)。

**第四步：创建 OAuth 客户端 ID - 选择“桌面应用”**

完成 OAuth 同意屏幕配置后（如果尚未配置，系统会提示您先完成），您可能需要再次点击“+ 创建凭据”->“OAuth 客户端 ID”。

*   **应用类型 (Application type):** 这是关键步骤。在应用类型列表中，务必选择“桌面应用”(Desktop app)。
*   **名称 (Name):** 为此 OAuth 客户端 ID 输入一个描述性的名称，以便您在控制台中轻松识别它（例如：“我的桌面应用 OAuth 客户端”）。

点击“创建”(CREATE) 按钮。

**第五步：获取客户端 ID 和客户端密钥**

1.  创建成功后，屏幕上会显示您的“客户端 ID”(Your Client ID) 和“客户端密钥”(Your Client Secret)。
2.  **非常重要：** 请立即复制并安全地保存这些信息。您的应用程序将使用这些 ID 和密钥来向 Google 身份验证服务标识自身，并请求访问用户数据。
3.  点击“确定”(OK) 关闭对话框。

**第六步：通过浏览器获取授权码 (Authorization Code)**

使用您在第五步中获得的客户端 ID，替换下方 URL 中的 `{YOUR_CLIENT_ID}` 占位符，并在您的网络浏览器中打开此 URL 以启动授权流程：

`https://accounts.google.com/o/oauth2/auth?client_id={YOUR_CLIENT_ID}&redirect_uri=http://localhost&scope=https://www.googleapis.com/auth/cloud-platform&response_type=code&access_type=offline&prompt=consent`

*   `client_id`: 您的客户端 ID。
*   `redirect_uri`: 必须与您在应用中使用的重定向 URI 匹配（此处使用 `http://localhost` ）。
*   `scope`: 请求的 API 权限范围（此处请求访问 Cloud Platform）。
*   `response_type=code`: 表示请求授权码。
*   `access_type=offline`: 请求刷新令牌，以便在用户离线时也能刷新访问令牌。
*   `prompt=consent`: 确保用户每次都看到同意屏幕（可选，但推荐首次使用）。

在浏览器中打开此 URL 后，Google 会提示您登录（如果尚未登录）并授权您的应用访问请求的权限。同意后，浏览器将被重定向到指定的 `redirect_uri` (`http://localhost`)，并在 URL 的查询参数中包含授权码。

如果一切顺利，您浏览器地址栏中的 URL 应该类似于以下格式：

`http://localhost/?code={YOUR_AUTHORIZATION_CODE}&scope=https://www.googleapis.com/auth/cloud-platform`

URL 中的 `{YOUR_AUTHORIZATION_CODE}` 就是您需要的授权码。请复制并保存此授权码，它只能使用一次且有效期很短。

**第七步：使用授权码交换访问令牌和刷新令牌**

打开一个终端或命令行界面，使用 `curl` 命令向 Google 的令牌端点发送 POST 请求，以使用您获得的授权码交换访问令牌 (access_token) 和刷新令牌 (refresh_token)。

将 `{YOUR_AUTHORIZATION_CODE}`、`{YOUR_CLIENT_ID}` 和 `{YOUR_CLIENT_SECRET}` 替换为实际的值：

```bash
curl -X POST \
  https://oauth2.googleapis.com/token \
  -d code={YOUR_AUTHORIZATION_CODE} \
  -d client_id={YOUR_CLIENT_ID} \
  -d client_secret={YOUR_CLIENT_SECRET} \
  -d redirect_uri=http://localhost \
  -d grant_type=authorization_code
```

*   `code`: 您在第六步中获得的授权码。
*   `client_id`: 您的客户端 ID。
*   `client_secret`: 您的客户端密钥。
*   `redirect_uri`: 必须与您在第六步中使用的重定向 URI 匹配。
*   `grant_type=authorization_code`: 表示您正在使用授权码流程。

如果请求成功，您将在终端中看到类似以下的 JSON 响应：

```json
{
  "access_token": "ya29...",       // 用于访问 API 的令牌
  "expires_in": 3599,             // 访问令牌的有效期（秒）
  "scope": "https://www.googleapis.com/auth/cloud-platform",
  "token_type": "Bearer",
  "refresh_token": "1//0ad..."    // 用于刷新访问令牌的令牌
}
```

请务必保存 `access_token` 和 `refresh_token`。`access_token` 是您用于调用 Vertex AI 或其他 GCP API 的凭据，通常有效期为一小时（3599 秒）。`refresh_token` 的有效期通常更长（例如一周或更久），用于在 `access_token` 过期后无需用户再次授权即可获取新的 `access_token`。

**第八步：使用刷新令牌获取新的访问令牌**

当您的 `access_token` 过期后，您可以使用保存的 `refresh_token` 来获取新的 `access_token`，而无需再次执行第六步的用户授权流程。

在终端中使用 `curl` 发送以下 POST 请求：

将 `{YOUR_STORED_REFRESH_TOKEN}`、`{YOUR_CLIENT_ID}` 和 `{YOUR_CLIENT_SECRET}` 替换为实际的值：

```bash
curl -X POST \
  https://oauth2.googleapis.com/token \
  -d grant_type=refresh_token \
  -d refresh_token={YOUR_STORED_REFRESH_TOKEN} \
  -d client_id={YOUR_CLIENT_ID} \
  -d client_secret={YOUR_CLIENT_SECRET}
```

*   `grant_type=refresh_token`: 表示您正在使用刷新令牌流程。
*   `refresh_token`: 您之前保存的刷新令牌。
*   `client_id`: 您的客户端 ID。
*   `client_secret`: 您的客户端密钥。

如果请求成功，您将收到一个包含新 `access_token` 的 JSON 响应：

```json
{
  "access_token": "ya29...",       // 新的访问令牌
  "expires_in": 3599,             // 新访问令牌的有效期（秒）
  "scope": "https://www.googleapis.com/auth/cloud-platform",
  "token_type": "Bearer"
  // 注意：通常不会在此响应中返回新的 refresh_token，除非旧的即将过期或 Google 的策略发生变化。
}
```