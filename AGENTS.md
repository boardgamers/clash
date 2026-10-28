# Agent instructions

## BGS publication and Git delivery

When the user asks to “push”, publish the changes to BGS as well as pushing the
matching source to the release branch.

Whenever changes are published to BGS, commit the corresponding source, tests,
dependency/lockfile changes and version bumps, then push them to this repository's
`main` or `master` release branch in the same task. A BGS upload or a push only to
a feature branch does not complete delivery. This is standing authorization to
commit and push published changes without asking for separate confirmation.

Fetch and integrate the latest release-branch changes, run the relevant repository
checks, and push without force. Update any public mirrors required by this repo's
existing workflow too. Keep credentials, generated artifacts excluded by the repo,
and unrelated unfinished work out of the commit. Verify the remote branch contains
the delivered commit and report any blocker instead of claiming delivery.
