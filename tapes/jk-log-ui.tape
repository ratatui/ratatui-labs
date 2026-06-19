Output target/betamax/command-palette.gif
Output target/betamax/command-palette.png
Output target/betamax/command-palette.json

Require cargo

Set Shell "bash"
Set Theme "GitHub Dark"
Set FontSize 24
Set Width 1000
Set Height 620
Set Margin 12
Set WindowBar Colorful
Set BorderRadius 8
Set TypingSpeed 35ms
Set CursorBlink false
Set WaitTimeout 30s

Hide
Type "cd /Users/joshka/local/ratatui-labs/default"
Enter
Wait
Type "cargo build --quiet --example command-palette && printf 'built-command-palette\n'"
Enter
Wait+Screen "built-command-palette"
Show

Type "cargo run --quiet --example command-palette"
Enter
Wait+Screen "jk log actions"
Screenshot target/betamax/command-palette-open.png
State target/betamax/command-palette-open.json
Sleep 2s

Type "theme"
Sleep 500ms
Wait+Screen "Switch theme"
Screenshot target/betamax/command-palette-filtered.png
Sleep 2s

Down
Sleep 500ms
Enter
Wait+Screen "Collecting input"
Screenshot target/betamax/command-palette-input.png
Sleep 2s

Backspace 5
Type "open"
Sleep 500ms
Wait+Screen "Open log"
Enter
Wait+Screen "invoked log.open"
Screenshot target/betamax/command-palette-invoked.png
State target/betamax/command-palette-invoked.json
Sleep 5s

Escape
