# Security policy

## Supported versions

Only the latest release gets fixes. kubelint releases often, so upgrading is the fix.

## Reporting a vulnerability

Please do not open a public issue. Use GitHub's private reporting instead: open the
repository's **Security** tab and choose **Report a vulnerability**.

Include the kubelint version (`kubelint --version`), the command you ran and, if
possible, a manifest that shows the problem.

kubelint reads files and prints findings. It never talks to a cluster or the network, so
the most likely issues are crashes or hangs on crafted YAML, and files read outside what
it was given.
