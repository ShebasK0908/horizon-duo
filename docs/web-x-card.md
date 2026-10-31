# Prepare a Horizon Player Card candidate

Generate a share page after building the browser client. The share page contains
static metadata and a normal play link. The Player Card URL points directly to
the existing game, which imports the player's resources before joining the world.

```sh
python3 tools/build_web.py
python3 tools/web_card.py \
  --player-url 'https://tests-mac-studio.tail935cc1.ts.net:8491/?world=webproof&bridge=wss%3A%2F%2Ftests-mac-studio.tail935cc1.ts.net%3A8492' \
  --image-url 'https://tests-mac-studio.tail935cc1.ts.net:8491/player-card.png'
```

The command writes ignored `web/share.html`. Open the private candidate at
`https://tests-mac-studio.tail935cc1.ts.net:8491/share.html` while connected to the
tailnet. **Play Horizon** opens the configured world. Both URL arguments require
HTTPS and reject credentials and whitespace. Use `--title` to change the title
and `--output` to write a different file.

The generator emits `twitter:card=player`, the absolute player URL, 720x720 player
dimensions, title, description, and image tags. Open Graph metadata supplies an
image preview for other readers. These tags are a candidate based on
[Rosebud's implementation report](https://lab.rosebud.ai/engineering/making-rosebud-games-playable-in-x-posts).
The old official Player Card URL redirects to X's developer overview. No current
X acceptance contract or Horizon post has been verified.

## Regenerate the original preview

`web/player-card.svg` is original geometric artwork. `web/player-card.png` is its
1200x630 Chrome rendering. Neither file contains imported game resources.

Set `HORIZON_PLAYWRIGHT` to your Playwright installation before running:

```sh
node - <<'JS'
const {chromium}=require(process.env.HORIZON_PLAYWRIGHT || 'playwright');
const fs=require('node:fs');
(async()=>{
  const browser=await chromium.launch({channel:'chrome',headless:true});
  try {
    const page=await browser.newPage({viewport:{width:1200,height:630},deviceScaleFactor:1});
    await page.setContent('<style>body{margin:0}svg{display:block}</style>'+fs.readFileSync('web/player-card.svg','utf8'));
    await page.screenshot({path:'web/player-card.png'});
  } finally {await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
JS
```

## Prepare public validation

The private candidate does not establish X crawler access or inline execution.
Before a real-post check, prepare these surfaces:

1. Host the browser files, generated share page, and preview at public HTTPS URLs.
   Regenerate the card with those URLs. Keep `.local/` and the private resource
   ZIP outside the public document root.
2. Expose a TLS WebSocket endpoint for a configured world bridge. Allow the
   game's public origin in that bridge. The browser connects from its own
   origin, including when another site embeds the game.
3. Preserve the bridge path `/world/WORLD`. The browser replaces the bridge
   URL's path with that value. A proxy must forward that full path.
4. Allow X to frame the game. An `X-Frame-Options: DENY` or `SAMEORIGIN` header
   blocks cross-origin framing. Any `frame-ancestors` policy must allow the
   actual X ancestor chain. Test the deployed headers and sandbox in the real
   client rather than assuming the local iframe's configuration matches X.
5. Fetch the public share HTML and preview without tailnet access or cookies.
   Check the metadata in the HTTP response without executing JavaScript.
6. With authorization, post that exact share URL and inspect the resulting card
   and actual gameplay in the selected X client. Record the post and client.

The local cross-origin iframe check proves one Chrome sandbox configuration.
The private share-page check proves metadata retrieval and ordinary play-link
navigation. Neither result substitutes for the public and X checks above.
Public hosting and posting have not occurred.
