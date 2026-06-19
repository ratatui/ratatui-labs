Output target/betamax/renderers/fullscreen.gif
Output target/betamax/renderers/fullscreen.png
Output target/betamax/renderers/fullscreen.json

Require cargo

Set Shell "bash"
Set Theme "GitHub Dark"
Set Width 1200
Set Height 620
Set WindowBar Colorful
Set BorderRadius 8
Set TypingSpeed 35ms
Set CursorBlink false
Set WaitTimeout 30s

Hide
Type "cd /Users/joshka/local/ratatui-labs/default"
Enter
Wait
Type "cargo run --quiet -p ratatui-command-palette --example command-palette -- --renderer fullscreen"
Enter
Wait+Screen "renderer: fullscreen"
Show
Screenshot target/betamax/renderers/fullscreen-open.png
State target/betamax/renderers/fullscreen-open.json
Sleep 2s

End
Sleep 500ms
Wait+Screen "Quit"
Screenshot target/betamax/renderers/fullscreen-scrolled.png
Sleep 2s

Escape
Wait
