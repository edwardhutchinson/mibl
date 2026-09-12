# Issue tracker

Issues and specs live in GitHub Issues for edwardhutchinson/mibl.
Use the gh CLI from this clone.

- Create: gh issue create --title "..." --body-file <file>
- Read: gh issue view <number> --comments
- List: gh issue list --state open
- Comment: gh issue comment <number> --body-file <file>
- Label: gh issue edit <number> --add-label "<label>"
- Remove label: gh issue edit <number> --remove-label "<label>"
- Close: gh issue close <number>

Write multiline bodies to a file and pass --body-file.

When a skill says to publish to the issue tracker, create a GitHub issue.
When it says to fetch a ticket, read the issue and its comments.

## Pull requests as a triage surface

PRs as a request surface: no.
