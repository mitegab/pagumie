# fish completion for ethdate
# Install to ~/.config/fish/completions/ethdate.fish or /usr/share/fish/vendor_completions.d/

set -l calendars ethiopic gregorian julian jdn rd unix

complete -c ethdate -f
complete -c ethdate -s d -l date -r -d 'Convert DATE' -a 'now today yesterday tomorrow'
complete -c ethdate -s f -l format -r -d 'strftime-style format'
complete -c ethdate -s F -l from -r -d 'Read dates in this calendar' -a "$calendars"
complete -c ethdate -s t -l to -r -d 'Print dates in this calendar' -a "$calendars"
complete -c ethdate -s g -l gregorian -d 'Print Gregorian dates'
complete -c ethdate -s e -l ethiopic -d 'Print Ethiopian dates'
complete -c ethdate -l julian -d 'Print Julian dates'
complete -c ethdate -l jdn -d 'Print Julian Day Numbers'
complete -c ethdate -l rd -d 'Print fixed day numbers'
complete -c ethdate -l unix -d 'Print Unix timestamps'
complete -c ethdate -s I -l iso -d 'Print bare YYYY-MM-DD'
complete -c ethdate -s j -l json -d 'Print JSON'
complete -c ethdate -l waybar -d 'Print Waybar custom-module JSON'
complete -c ethdate -l polybar -d 'Print one status-bar line'
complete -c ethdate -l i3blocks -d 'Print one status-bar line'
complete -c ethdate -l locale -r -d 'Script for names' -a 'am\t"Amharic" en\t"Latin"'
complete -c ethdate -l era -r -d 'Year numbering' -a 'am\t"Amete Mihret" aa\t"Amete Alem"'
complete -c ethdate -l day-start -r -d 'When a day turns' -a 'dawn\t"06:00, traditional" midnight\t"00:00, civil"'
complete -c ethdate -s u -l utc -d 'Use UTC'
complete -c ethdate -l offset -r -d 'Fixed UTC offset' -a '+03:00 +00:00 -05:00'
complete -c ethdate -s T -l time -d 'Include the Ethiopian time of day'
complete -c ethdate -s c -l calendar -d 'Print a month grid'
complete -c ethdate -s y -l year -d 'Print a whole year'
complete -c ethdate -l holidays -d 'List the holidays of a year'
complete -c ethdate -l computus -d 'Print the Bahire Hasab table'
complete -c ethdate -l is-holiday -d 'Exit 0 if the date is a public holiday'
complete -c ethdate -s s -l stdin -d 'Read dates from standard input'
complete -c ethdate -s 0 -l null -d 'NUL-separated records'
complete -c ethdate -s q -l quiet -d 'No error messages'
complete -c ethdate -l no-color -d 'Never colourise'
complete -c ethdate -s h -l help -d 'Show help'
complete -c ethdate -s V -l version -d 'Show version'
