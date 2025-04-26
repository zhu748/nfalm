# Clewdr 部署到 huggingface space

简介

[Clewdr](https://github.com/Xerxes-2) 是一个 claude 的逆向 api 项目。拥有前端可以直接添加 cookie 和修改设置

## 开始

### 创建空间

1. 前往 https://hf.space 点击右侧的 **New space** 创建一个空间

---

2. ![](https://raw.githubusercontent.com/Goojoe/PicList/master/images/2025-04-26_15-14-31_651.avif)

---

3. 填写详情页

---

4. ![](https://raw.githubusercontent.com/Goojoe/PicList/master/images/2025-04-26_15-14-35_000.avif)

---

5. ![](https://raw.githubusercontent.com/Goojoe/PicList/master/images/2025-04-26_15-14-37_973.avif)

---

### 上传 Dockerfile

![image.png](https://raw.githubusercontent.com/Goojoe/PicList/master/images/2025-04-26_15-14-41_795.avif)

![image.png](https://raw.githubusercontent.com/Goojoe/PicList/master/images/2025-04-26_15-14-44_914.avif)

### [下载 Dockerfile.huggingface](https://github.com/Xerxes-2/clewdr/blob/master/Dockerfile.huggingface)

手动重命名为`Dockerfile`并上传
![image.png](https://raw.githubusercontent.com/Goojoe/PicList/master/images/2025-04-26_15-14-48_851.avif)

状态为 Running 后打开日志
![image.png](https://raw.githubusercontent.com/Goojoe/PicList/master/images/2025-04-26_15-14-52_190.avif)
![image.png](https://raw.githubusercontent.com/Goojoe/PicList/master/images/2025-04-26_15-14-56_188.avif)

LLM API Password 为请求 API 的密码
Web Admin Password 为前端管理密码

---

### API 地址

![](https://raw.githubusercontent.com/Goojoe/PicList/master/images/2025-04-26_15-14-58_660.avif)
![](https://raw.githubusercontent.com/Goojoe/PicList/master/images/2025-04-26_15-15-02_846.avif)

复制后加上`/v1`后缀即可在酒馆使用,为 claude 加反向代理

举例 API 地址

```
https://3v4pyve7-clewdr.hf.space/v1
```

认证令牌为上文的 Web Admin Password

如果你不想每次重启都要手动加 cookie,请继续看下面的**自定义变量**

### 自定义变量

![image.png](https://raw.githubusercontent.com/Goojoe/PicList/master/images/2025-04-26_15-15-05_524.avif)

![image.png](https://raw.githubusercontent.com/Goojoe/PicList/master/images/2025-04-26_15-15-08_059.avif)

配置如下

> 注意!!! 隐私信息比如 密码 Cookie 请填到 Secrets 里

简单配置

```
CLEWDR_COOKIE_ARRAY=[[sk-ant-sid01-MRM7m79xnS101emZmZvm-VU8ptQAA,sk-ant-sid01-MRM7m79xnS101emZmZvm-VU8ptQAA]]
CLEWDR_PASSWORD=your_secure_password
CLEWDR_ADMIN_PASSWORD=your_admin_password
```

完整默认配置文件

```
clewdr_cookie_array = [[]]
clewdr_wasted_cookie = [] #这个可以不用管
clewdr_ip = "127.0.0.1" #此项禁止加入变量
clewdr_port = 8484  #此项禁止加入变量
clewdr_enable_oai = false
clewdr_check_update = true
clewdr_auto_update = false
clewdr_password = "password"
clewdr_admin_password = "password"
clewdr_max_retries = 5
clewdr_pass_params = false
clewdr_preserve_chats = false
clewdr_skip_warning = false
clewdr_skip_restricted = false
clewdr_skip_non_pro = false
clewdr_use_real_roles = true
clewdr_custom_prompt = ""
clewdr_padtxt_len = 4000

```

### 更新

前往 Settings -> Factory rebuild 点击按钮即可
