#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
SOURCE="$REPO_ROOT/scripts/shell/skz-publish.zsh"
TARGET_DIR="${SKZ_SNIPPETS_DIR:-$HOME/skz/snippets}"
action="install"

usage() {
    printf '%s\n' '用法: install_skz_publish.sh [--uninstall] [--target-dir <DIR>] [--source <FILE>]'
    printf '%s\n' '只创建或删除符号链接，不修改 rc 文件；不覆盖普通文件或不匹配的符号链接。'
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --uninstall) action="uninstall"; shift ;;
        --target-dir|--source)
            if [[ $# -lt 2 || -z "$2" || "$2" == --* ]]; then
                printf '%s 需要非空参数值\n' "$1" >&2
                exit 2
            fi
            if [[ "$1" == --target-dir ]]; then TARGET_DIR="$2"
            else SOURCE="$2"
            fi
            shift 2
            ;;
        -h|--help) usage; exit 0 ;;
        *) printf '未知选项: %s\n' "$1" >&2; usage >&2; exit 2 ;;
    esac
done

LINK="$TARGET_DIR/skz-publish.zsh"
if [[ "$action" == uninstall ]]; then
    if [[ -L "$LINK" ]]; then
        rm -- "$LINK"
        printf '[ok] 已删除符号链接: %s\n' "$LINK"
    elif [[ -e "$LINK" ]]; then
        printf '[error] %s 存在但不是符号链接，拒绝删除\n' "$LINK" >&2
        exit 1
    else
        printf '[skip] %s 不存在，无需卸载\n' "$LINK"
    fi
    exit 0
fi

if [[ ! -f "$SOURCE" ]]; then
    printf '[error] 找不到 snippet 源: %s\n' "$SOURCE" >&2
    exit 1
fi
SOURCE="$(cd -- "$(dirname -- "$SOURCE")" && pwd -P)/$(basename -- "$SOURCE")"
mkdir -p -- "$TARGET_DIR"
TARGET_DIR="$(cd -- "$TARGET_DIR" && pwd -P)"
LINK="$TARGET_DIR/skz-publish.zsh"

if [[ -L "$LINK" ]]; then
    if [[ "$(readlink -- "$LINK")" == "$SOURCE" ]]; then
        printf '[skip] 符号链接已指向正确源: %s\n' "$LINK"
    else
        printf '[error] %s 是不匹配或失效的符号链接，拒绝覆盖；确认后先 --uninstall 再安装\n' "$LINK" >&2
        exit 1
    fi
elif [[ -e "$LINK" ]]; then
    printf '[error] %s 已存在且不是符号链接，拒绝覆盖\n' "$LINK" >&2
    exit 1
else
    ln -s -- "$SOURCE" "$LINK"
    printf '[ok] 已创建符号链接: %s -> %s\n' "$LINK" "$SOURCE"
fi

printf '\n将下面一行加入 ~/.zshrc 或 ~/.bashrc；路径是本次实际安装目标：\n\n'
printf 'if [[ -f %q ]]; then source %q; fi\n' "$LINK" "$LINK"
printf '\n重新打开 shell 或手工执行上面的 source 行；脚本不会修改 rc 文件。\n'
