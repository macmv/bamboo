#! /bin/sh

function prefixwith {
    local PREF="${1//\//\\/}" # replace / with \/
    shift
    local CMD=("$@")
    ${CMD[@]} 1> >(sed "s/^/${PREF}/") 2> >(sed "s/^/${PREF}/" 1>&2)
}

prefixwith "[SERVER] " cargo run --bin bb_server --release &
prefixwith "[PROXY] " cargo run --bin bb_proxy --release &

wait
