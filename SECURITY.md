# Security and privacy

Security maintenance targets the latest source revision. This project is a
personal desktop tool, not a hardened service for the public internet.

## Remote access

The remote host is disabled by default. Set `HEART_ENABLE_REMOTE=1` before
starting the desktop app to enable it. Enabling binds HTTP to all IPv4
interfaces (TCP 37218–37228) and discovery to UDP 37219. Permit it only on
trusted networks. Discovery advertises a generic service name rather than the
operating-system computer name.

Pairing uses a six-digit one-time code, a failed-attempt cooldown, and a
256-bit session token. Tokens remain in memory on the PC and are invalidated
on restart or pairing reset. The Android app stores its token in app-private
preferences with backup disabled. Media query tokens are accepted only by
screen endpoints; other authenticated APIs use bearer headers.

The protocol is HTTP and does not itself encrypt the library, screen or input.
Use a trusted LAN or an encrypted private VPN. Do not forward these ports from
a public router. A paired client can see library metadata and the entire
desktop and send mouse/keyboard input; only pair devices you control.

## Local data

SQLite data, notes, tags and remembered library paths are not encrypted.
Screenshots, logs and error messages may contain private paths or content.
The source archive includes none of your runtime database or connection data.
Git ignore rules reduce accidental commits but do not remove already tracked
files or old Git history. The release checker is a heuristic, not proof of
the absence of every secret.

Microphone listening requires the user's action. Web Speech API availability
and speech processing depend on the installed runtime; the runtime may use a
network speech service. Do not assume offline speech recognition.

## Reporting

Use GitHub's private security advisories when enabled by the repository owner.
Do not publish tokens, addresses, screenshots of private media or real data in
a public issue. No independent penetration test has been performed.
