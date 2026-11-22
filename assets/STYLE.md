# whylog writing style

Everything you write through whylog (notes, decisions, summaries, compaction output) follows
these rules. The goal is writing that reads like a person who knows the project talking
straight to the next one.

## Voice

- Be direct. Say what happened and what you decided. No warm-up.
- Own it. "I removed the retries", not "The retries were removed". Active voice.
- No hedging. Drop "I think", "perhaps", "it might be possible that". If you're not sure,
  say exactly what you're not sure about.
- No apologizing, no disclaimers, no softening before a criticism.
- No corporate or academic filler: "Furthermore", "It should be noted", "leverage",
  "robust", "seamless", "in order to".
- Keep it clean. No profanity.

## Shape

- Short sentences. Short paragraphs, 1-4 sentences. Fragments are fine when they land.
- **No em-dashes. Ever.** Use a period, a comma, a colon or parentheses. (whylog turns any
  em-dash it receives into a plain hyphen.)
- Contractions are fine: don't, can't, it's, we're.
- Use specific numbers and names: "cut load time from 2.8s to 40ms", not "made it faster".
  Name the file, the function, the setting.
- Bold the thing that must not be missed. Don't bold whole sentences.

## Decisions

Format: `- **what you decided** - *why*`

- The what is a rule someone can follow: "No automatic retries in the HTTP client".
- The why is the real reason, in plain words: "they caused duplicate writes".
- One decision per line. If it needs a paragraph, it's probably two decisions.
- Reverted or rejected ideas are decisions too: `**No blurred pause menu** - *tried it, it looked bad*`.

## Notes

- Write what you found and why it matters, not a play-by-play of every command.
- Good: "The cache key ignored the locale, so French users got English pages."
- Bad: "I looked at the cache and then I checked some things and found an issue."

## Summaries

- One line. What changed, in plain words: "Removed client retries and added idempotency keys".
