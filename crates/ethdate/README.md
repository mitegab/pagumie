# `ethdate`

The Ethiopian calendar as a Unix command-line tool. No runtime dependencies.

```console
$ ethdate
ረቡዕ ፣ 1 መስከረም 2017 ዓ.ም.

$ ethdate -F gregorian --iso 2024-09-11
2017-01-01

$ ethdate -g --iso 2015-13-06        # the sixth epagomenal day
2023-09-11

$ ethdate --json | jq -r .ethiopic.month_name
መስከረም

$ seq 0 6 | sed 's/^/+/' | ethdate -  # the coming week
$ ethdate --is-holiday -q || make     # 0 = holiday, 1 = ordinary day
```

* `--format` with a `strftime(3)`-style grammar, including `%G` for Ge'ez numerals
* `--json`, `--waybar` and `--polybar` output for scripts and status bars
* `--calendar` / `--year`: a `cal(1)` grid in which Ṗagumēn is a real month
* `--holidays`, `--computus`: the year's holidays and the Baḥire Ḥasab table
* meaningful exit codes, NUL-separated records, broken pipes handled

See [`man/ethdate.1`](man/ethdate.1) for the full manual, and
[`completions/`](completions) for bash, zsh and fish completions.
