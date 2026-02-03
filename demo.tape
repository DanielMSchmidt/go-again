# VHS Demo for go-again
# Run with: vhs demo.tape
# Requires: vhs (brew install vhs), go, go-again in PATH

Output demo.gif
Set Theme "Dracula"
Set FontSize 16
Set Width 900
Set Height 500
Set Padding 20

# Setup - start in example project
Type "cd testdata/example-project"
Enter

# Show we're in a Go project
Type "# Let's run the tests and see what fails..."
Enter
Sleep 1s

# Run tests and remember failures
Type "go test ./... | go-again remember"
Enter
Sleep 3s

# List the failures
Type "# Now let's see what go-again remembered:"
Enter
Type "# (Note: remembering is project & branch aware)"
Enter
Sleep 500ms
Type "go-again list"
Enter
Sleep 2s

# Show the run command
Type "# We can re-run just those failing tests:"
Enter
Type "# (Hint: there is also go-again select and watch)"
Enter
Sleep 500ms
Type "go-again run"
Enter
Sleep 3s

# Clear when done
Type "# When done, clear the list:"
Enter
Sleep 500ms
Type "go-again clear"
Enter
Sleep 1s

Type "go-again list"
Enter
Sleep 2s

Type "# That's it!"
Enter
Sleep 2s
