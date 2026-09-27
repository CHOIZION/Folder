# Contributing

Use synthetic media names, tags, paths and screenshots in examples and tests.
Do not attach real libraries, databases, device addresses, tokens, signing keys
or environment files to commits or issue reports.

Run `npm ci`, `npm run verify` and `npm run build` on Windows before submitting
a pull request. For Android changes, also run `android-remote/build-apk.ps1`
and test on an Android device. Describe behavior changes and checks actually run.

Keep scanning, persistence, desktop presentation and remote transport separate;
see `ARCHITECTURE.md`. Preserve existing copyright and third-party notices.

Report security issues using a private repository advisory if the maintainer
has enabled it. Never post access tokens or private data in a public issue.
