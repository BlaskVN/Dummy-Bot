# English five-letter Word Set

Source: Wordnik `wordlist-20210729.txt` at commit
`46e6215d0f90356afe9c8ba4be347e7e98cb425c`:

https://github.com/wordnik/wordlist/blob/46e6215d0f90356afe9c8ba4be347e7e98cb425c/wordlist-20210729.txt

Retrieved 2026-08-10. Wordnik describes the source as an open-source English
word list for game developers. Redistribution is under the MIT License; the
required notice is preserved in `LICENSE.wordnik`.

Upstream SHA-256:
`bfd1b4eb4ade1ba81e84c7e24248b9a1aecec9d9baa427453b367a83e30e0451`.
The derived `allowed.txt` SHA-256 is
`2f3df71fb745ec50271848d018ed8d79975a73559a27f3e5852773ad6ca593a8`.
It can be regenerated from the pinned download with:

```sh
sed -n 's/^"\([a-z]\{5\}\)"$/\1/p' wordlist-20210729.txt | LC_ALL=C sort -u > allowed.txt
```

`allowed.txt` is reproducibly derived by removing the source JSON quotes,
retaining only unique lowercase five-letter ASCII entries, and sorting them.
`answers.txt` is a deliberately smaller, human-reviewed selection of familiar,
neutral words from `allowed.txt`. Offensive, derogatory, sexual, graphic, and
identity-targeting terms are excluded from answers. They remain in the allowed
guess dictionary when present in the upstream source so the bot does not need
to define or continually expand a speech-policy filter for private guesses.

This data is independent of and does not copy the proprietary Wordle answer
list. Words stay English; only the surrounding bot interface is localized.
