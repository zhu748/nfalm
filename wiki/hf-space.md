# Clewdr 部署到 huggingface space

简介
[Clewdr](https://github.com/Xerxes-2) 是一个 claude 的逆向 api 项目。拥有前端可以直接添加 cookie 和修改设置

## 开始

### 创建空间

1. 前往 https://hf.space 点击右侧的 **New space** 创建一个空间

---

2. ![](https://i.imgur.com/Tfijg4d.png)

---

3. 填写详情页

---

4. ![](https://i.imgur.com/To9YA6H.png)

---

5. ![](https://i.imgur.com/c3QqkhQ.png)

---

### 上传 Dockerfile

![image.png](https://i.imgur.com/0LrsDTz.png)

![image.png](https://i.imgur.com/NU0tcsQ.png)

### [下载 Dockerfile.huggingface](https://github.com/Xerxes-2/clewdr/blob/master/Dockerfile.huggingface)

手动重命名为`Dockerfile`并上传
![image.png](https://i.imgur.com/tK02hTe.png)

状态为 Running 后打开日志
![image.png](https://i.imgur.com/DJIsBy1.png)
![image.png](https://i.imgur.com/bPNc8PU.png)

LLM API Password 为请求 API 的密码
Web Admin Password 为前端管理密码

---

### 前端地址

复制你的项目名

![image.png](https://i.imgur.com/dtwRXYk.png)

它应该是这样的:`3v4pyve7/clewdr2`
现在将`/`改成`-`后面加上

```
.hf.space
```

拼接后就可以直接访问了
https://3v4pyve7-clewdr2.hf.space

认证令牌为上文的 Web Admin Password

如果你不想每次重启都要手动加 cookie,请继续看下面的**自定义变量**

### 自定义变量

![image.png](https://i.imgur.com/G27CuYM.png)

![image.png](https://i.imgur.com/5lolAT1.png)

配置如下

注意!!! 隐私信息比如密码请填到 Secrets 里

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
