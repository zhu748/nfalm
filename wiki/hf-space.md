# Clewdr 部署到 huggingface space
简介
[Clewdr](https://github.com/Xerxes-2) 是一个 claude的逆向api项目。拥有前端可以直接添加cookie和修改设置

## 开始
### 创建空间
1. 前往 https://hf.space 点击右侧的 **New space** 创建一个空间
2. ![](https://i.imgur.com/Tfijg4d.png)
3. 填写详情页
4. ![](https://i.imgur.com/To9YA6H.png)
5. ![](https://i.imgur.com/c3QqkhQ.png)

### 上传Dockerfile
![image.png](https://i.imgur.com/0LrsDTz.png)

![image.png](https://i.imgur.com/NU0tcsQ.png)

### [下载 Dockerfile.huggingface](https://github.com/Xerxes-2/clewdr/blob/master/Dockerfile.huggingface)
手动重命名为`Dockerfile`并上传
![image.png](https://i.imgur.com/tK02hTe.png)

状态为Running后打开日志
![image.png](https://i.imgur.com/DJIsBy1.png)
![image.png](https://i.imgur.com/bPNc8PU.png)

LLM API Password为请求API的密码
Web Admin Password为前端管理密码

### 前端添加cookie(饼干)
复制你的项目名
![image.png](https://i.imgur.com/dtwRXYk.png)

它应该是这样的:`3v4pyve7/clewdr2`
现在将`/`改成`-`后面加上
```
.hf.space
```

拼接后就可以直接访问了
https://3v4pyve7-clewdr2.hf.space
认证令牌为上文的Web Admin Password
教程结束,以下为自定义设置,可以不管

### 自定义密码
![image.png](https://i.imgur.com/G27CuYM.png)

![image.png](https://i.imgur.com/5lolAT1.png)

配置如下
Name为变量名 Value是密码
CLEWDR_PASSWORD
your_secure_password

CLEWDR_ADMIN_PASSWORD
your_admin_password


