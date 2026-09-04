# Security

HollowChat is built and run by one person. That means reports get read by
someone who can actually fix them, and it means there is no team on call — so
please give me a little time before going public.

## Reporting a vulnerability

Use GitHub's private reporting: **[Report a
vulnerability](https://github.com/dst-port/hollow-chat/security/advisories/new)**
(Security → Advisories on the repository). That keeps the details out of public
issues until there's a fix.

If that doesn't work for you, open a regular issue saying only that you have a
security report and how to reach you — no details in the issue itself.

What helps most:

- What you did, and what happened that shouldn't have.
- Whether it needs an account, and whose (yours, a server admin's, the
  operator's).
- Anything that shows it working — a request, a snippet, a short recording.

You don't need a proof-of-concept exploit or a severity rating. A clear
description of the wrong behaviour is plenty.

**Please don't** test against `hollowchat.org` in ways that would affect other
people: no brute-forcing accounts you don't own, no load testing, no reading or
modifying anyone else's data. Run it locally (`docker compose up`) instead — the
whole thing is in this repository. If you can only reproduce it in production,
say so and stop there; I'd rather reproduce it myself than have someone poke at
other people's messages.

I'll confirm I've read your report, tell you what I think of it, and let you
know when it's fixed and released. You're credited in the changelog unless you'd
rather not be.

## What's in scope

Anything in this repository: the Rust server, the SvelteKit client, the Tauri
desktop app, the browser extension, the deployment configuration.

Especially interesting:

- **Anything that lets the server read message content.** Messages are
  end-to-end encrypted, and the server is supposed to be unable to read them.
  That's the strongest claim the project makes, so it's the one most worth
  attacking. Note the client pins a contact's identity key on first contact and
  exposes a safety number for out-of-band comparison — if you can get around
  either, I want to know.
- One user reading, changing, or deleting another's data.
- Escalating inside a server beyond the permissions you were granted.
- Anything that runs attacker-controlled code in the app or the desktop shell.

## Known limits, so you don't spend time on them

These are understood trade-offs, not oversights — reports about them are
welcome but won't be treated as vulnerabilities:

- **No email, so no account recovery.** Accounts are a username and a generated
  password by design. Lose both and the account is gone; there is nothing to
  reset.
- **The server sees metadata.** Who talks to whom, when, and how much. Message
  bodies and attachments are encrypted; the shape of the conversation isn't.
- **Sessions carry no device information.** The session list deliberately
  records no IP or user agent, which is a privacy choice that costs you some
  ability to recognise a stranger's session.
- **A self-hosted deployment is only as good as its configuration.**
  `deploy/README.md` has the checklist; misconfigured self-hosts aren't
  vulnerabilities in the software.

## Supported versions

The latest release, and whatever `hollowchat.org` is running. There are no
backports to older versions.
