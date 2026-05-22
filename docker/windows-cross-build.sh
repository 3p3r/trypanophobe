#!/usr/bin/env bash
# ort links DirectML.lib / PathCch.lib; xwin ships directml.lib / pathcch.lib (Linux is case-sensitive).
set -euo pipefail

cargo xwin cache xwin

xwin_root="${XWIN_ROOT:-${HOME}/.cache/cargo-xwin/xwin}"
for lib_dir in "${xwin_root}/sdk/lib/um"/*/; do
	[[ -d "${lib_dir}" ]] || continue
	if [[ -f "${lib_dir}pathcch.lib" && ! -e "${lib_dir}PathCch.lib" ]]; then
		ln -sfn pathcch.lib "${lib_dir}PathCch.lib"
	fi
	if [[ -f "${lib_dir}directml.lib" && ! -e "${lib_dir}DirectML.lib" ]]; then
		ln -sfn directml.lib "${lib_dir}DirectML.lib"
	fi
done

exec cargo xwin build --release --target x86_64-pc-windows-msvc "$@"
