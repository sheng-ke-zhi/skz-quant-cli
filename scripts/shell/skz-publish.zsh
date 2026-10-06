#!/usr/bin/env zsh

skz-publish() {
    local dir="" code="" memo="" requested_run="" arg value
    while [[ $# -gt 0 ]]; do
        arg="$1"
        case "$arg" in
            -m|--memo|--run-id)
                if [[ $# -lt 2 || -z "$2" ]]; then
                    printf '%s 需要非空参数值\n' "$arg" >&2
                    return 2
                fi
                case "$2" in
                    --run-id|--memo|-m|-h|--help)
                        printf '%s 缺少参数值\n' "$arg" >&2
                        return 2
                        ;;
                esac
                if [[ "$arg" == --run-id ]]; then
                    requested_run="$2"
                else
                    memo="$2"
                fi
                shift 2
                ;;
            -h|--help)
                printf '%s\n' '用法: skz-publish <explore_dir> <strategy_code> [--run-id <run_id>] [-m|--memo <note>]'
                printf '%s\n' '默认递归查找同名 TOML；完全相同的副本只用 strategy register 登记一次（暂停态，不跑回测）。'
                printf '%s\n' '--run-id 仅用于显式平台候选 promote，限定同名子目录；--memo 仅与 --run-id 联用。'
                return 0
                ;;
            -*) printf '未知选项: %s\n' "$arg" >&2; return 2 ;;
            *)
                if [[ -z "$dir" ]]; then dir="$arg"
                elif [[ -z "$code" ]]; then code="$arg"
                else printf '多余参数: %s\n' "$arg" >&2; return 2
                fi
                shift
                ;;
        esac
    done

    if [[ -z "$dir" || -z "$code" ]]; then
        printf '%s\n' '用法: skz-publish <explore_dir> <strategy_code> [--run-id <run_id>] [-m|--memo <note>]' >&2
        return 2
    fi
    for value in "$code" "$requested_run"; do
        [[ -n "$value" ]] || continue
        case "$value" in
            .|..|-*|*/*|*\\*|*'*'*|*'?'*|*'['*|*']'*|*$'\n'*|*$'\r'*)
                printf '策略代码或 run ID 不是安全的单个文件名: %s\n' "$value" >&2
                return 2
                ;;
        esac
    done
    if [[ "$requested_run" == .runs ]]; then
        printf '%s\n' '.runs 不是可提交的 run 目录' >&2
        return 2
    fi
    if [[ -n "$memo" && -z "$requested_run" ]]; then
        printf '%s\n' '默认 strategy register 不支持 --memo；登记成功后先读回现有笔记，再用 skz strategy memo 写入完整笔记' >&2
        return 2
    fi
    if [[ ! -d "$dir" || ! -r "$dir" || ! -x "$dir" ]]; then
        printf '探索结果目录不存在或不可访问: %s\n' "$dir" >&2
        return 1
    fi
    dir="$(cd -- "$dir" && pwd -P)" || return 1

    local selected="" search_dir="$dir" hit="" count=0 scan_completed=0 conflict=0 compare_error=0 compare_result=0
    if [[ -n "$requested_run" ]]; then
        search_dir="$dir/$requested_run"
        if [[ ! -d "$search_dir" || -L "$search_dir" ]]; then
            printf '指定 run 目录不存在或为符号链接: %s\n' "$search_dir" >&2
            return 1
        fi
    fi
    while IFS= read -r -d '' hit; do
        if [[ -z "$hit" ]]; then
            scan_completed=1
            continue
        fi
        count=$((count + 1))
        printf '[match] toml=%s\n' "$hit" >&2
        if [[ -z "$selected" ]]; then
            selected="$hit"
        elif cmp -s -- "$selected" "$hit"; then
            printf '[duplicate] 内容完全相同，合并: %s\n' "$hit" >&2
        else
            compare_result=$?
            if [[ "$compare_result" -eq 1 ]]; then
                conflict=1
                printf '[conflict] 内容不同: %s\n' "$hit" >&2
            else
                compare_error=1
                printf '[error] 无法比较文件: %s\n' "$hit" >&2
            fi
        fi
    done < <(find "$search_dir" -type d -name .runs -prune -o -type f -name "$code.toml" -print0 && printf '\0')
    if [[ "$scan_completed" -ne 1 || "$compare_error" -ne 0 ]]; then
        printf '%s\n' '递归扫描或文件比较失败，未提交' >&2
        return 1
    fi

    if [[ "$count" -eq 0 ]]; then
        printf '未在 %s 下递归找到普通文件 %s.toml\n' "$search_dir" "$code" >&2
        return 1
    fi
    if [[ "$conflict" -ne 0 ]]; then
        printf '命中 %s 个同名文件但内容不同，未提交；请缩小 explore_dir 或先处理配置冲突\n' "$count" >&2
        return 2
    fi

    if [[ -z "$requested_run" ]]; then
        printf '[submit] code=%s toml=%s；%s 份相同配置仅登记一次，暂停态、不跑回测\n' "$code" "$selected" "$count" >&2
        command skz strategy register "$selected"
    else
        printf '[submit] platform_experiment=%s code=%s；请求受理不等于入库成功\n' "$requested_run" "$code" >&2
        if [[ -n "$memo" ]]; then
            command skz promote start "$requested_run" "$code" --memo "$memo"
        else
            command skz promote start "$requested_run" "$code"
        fi
    fi
}
