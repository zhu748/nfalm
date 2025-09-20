# ClewdR 数据库持久化配置

ClewdR 默认把配置、Cookie、API Key 等状态写入本地的 `clewdr.toml`。当你需要在多实例场景同步状态，或者希望在容器重启后自动恢复，可以启用数据库持久化模式（SQLite / Postgres / MySQL）。

## 功能说明

- `persistence.mode` 支持 `file`（默认）、`sqlite`、`postgres`、`mysql`
- 启用数据库模式后，SeaORM 会把配置、Cookie、废弃原因、Key 等表结构统一迁移到数据库
- 所有前端写入接口都会在操作前执行健康检查，连接异常时会返回“Database storage is unavailable”

## 编译启用数据库驱动

> **重要提示**：数据库相关代码通过特性开关控制，预编译包如果未启用对应特性，会自动退回文件存储模式。

- 从源码构建时附加 `db-*` 特性，例如：

  ```bash
  cargo build --release --no-default-features --features "embed-resource,xdg,db-postgres"
  ```

- `db-sqlite`、`db-postgres`、`db-mysql` 会自动启用基础的 `db` 特性，可组合启用多个驱动
- 自定义 Docker 镜像时需要修改 `cargo build` 命令加入以上特性；仓库默认 Dockerfile 仅包含文件模式

## `clewdr.toml` 配置示例

```toml
[persistence]
mode = "postgres"               # sqlite | postgres | mysql
database_url = "postgres://user:pass@db:5432/clewdr"

# SQLite 可选：使用绝对路径自动展开为 sqlite://...?... 并创建目录
# sqlite_path = "/var/lib/clewdr/clewdr.db"
```

- Postgres / MySQL 模式必须提供 `database_url`（可附带 SSL 参数等）
- SQLite 如果未设置 `database_url`，会把 `sqlite_path` 展开为 `sqlite://PATH?mode=rwc`，并在允许的情况下创建父目录
- 根据部署需求，可配合挂载持久化卷，确保 SQLite 文件不会随容器删除

## 环境变量配置

ClewdR 使用 Figment 双下划线映射嵌套字段，可以通过环境变量快速切换：

```bash
export CLEWDR_PERSISTENCE__MODE=sqlite
export CLEWDR_PERSISTENCE__SQLITE_PATH=/var/lib/clewdr/clewdr.db
# 或者使用服务器数据库
export CLEWDR_PERSISTENCE__DATABASE_URL="postgres://user:pass@db/clewdr"
```

## 运行注意事项

- 首次连接会自动执行 SeaORM 迁移，请确保数据库用户具备建表和建索引权限
- 任何 Cookie/Key 写入接口调用前都会执行 `/api/storage/status`，连通性异常会直接拒绝写入
- 通过 `GET /api/storage/status` 可观察健康状态，管理员令牌还可调用 `/api/storage/import`、`/api/storage/export` 与文件互相同步
- 未启用对应 `db-*` 特性的二进制无法进入数据库模式，即使 `persistence.mode` 设置为数据库也会退回文件模式
- 在只读文件系统或重复部署环境下，若不需要落地 `clewdr.toml` 可在配置中设置 `no_fs = true`

