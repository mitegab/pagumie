# bash completion for ethdate
# Install to /usr/share/bash-completion/completions/ethdate

_ethdate() {
    local cur prev
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    local calendars="ethiopic gregorian julian jdn rd unix"

    case "$prev" in
        -F|--from|-t|--to)
            COMPREPLY=( $(compgen -W "$calendars" -- "$cur") )
            return 0 ;;
        --locale)
            COMPREPLY=( $(compgen -W "am en" -- "$cur") )
            return 0 ;;
        --era)
            COMPREPLY=( $(compgen -W "am aa" -- "$cur") )
            return 0 ;;
        --day-start)
            COMPREPLY=( $(compgen -W "dawn midnight" -- "$cur") )
            return 0 ;;
        -d|--date)
            COMPREPLY=( $(compgen -W "now today yesterday tomorrow" -- "$cur") )
            return 0 ;;
        -f|--format)
            # Offer the most useful format strings rather than nothing at all.
            COMPREPLY=( $(compgen -W "'%Y-%m-%d' '%A %-d %B %Y' '%G' '%g' '%J' '%R'" -- "$cur") )
            return 0 ;;
        --offset)
            COMPREPLY=( $(compgen -W "+03:00 +00:00 -05:00" -- "$cur") )
            return 0 ;;
        --calendar|--year|--holidays|--computus)
            return 0 ;;
    esac

    if [[ "$cur" == -* ]]; then
        local opts="--date --format --from --to --gregorian --ethiopic --julian --jdn --rd
                    --unix --iso --json --waybar --polybar --i3blocks --locale --era
                    --day-start --utc --offset --time --calendar --year --holidays
                    --computus --is-holiday --stdin --null --quiet --no-color --help --version"
        COMPREPLY=( $(compgen -W "$opts" -- "$cur") )
        return 0
    fi

    COMPREPLY=( $(compgen -W "now today yesterday tomorrow" -- "$cur") )
}
complete -F _ethdate ethdate
