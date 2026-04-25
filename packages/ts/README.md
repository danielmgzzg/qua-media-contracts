# @qua/media-contracts (TypeScript)

Generated TypeScript types + Ajv-based runtime validators for the Qua
media pipeline wire protocol.

## Generate

```sh
npm install
npm run gen      # writes src/index.ts from ../../schemas/v1/**
npm run build    # gen + tsc to dist/
npm run validate # validates examples (if any) against the schemas
```

## Use

```ts
import type { WsMessage, ClientMessage, Snapshot } from "@qua/media-contracts";

function handle(msg: WsMessage) {
  switch (msg.type) {
    case "snapshot":
      // msg is narrowed to the Snapshot variant
      console.log(msg.episode.title);
      break;
    case "worker_heartbeat":
      console.log(`${msg.worker_id}: ${msg.cpu_percent}%`);
      break;
    // ...
  }
}
```

For runtime validation in dev (recommended; strip in prod):

```ts
import Ajv2020 from "ajv/dist/2020.js";
import serverSchema from "@qua/media-contracts/schemas/v1/ws/server.schema.json";

const ajv = new Ajv2020({ strict: false });
const validate = ajv.compile(serverSchema);

ws.addEventListener("message", (ev) => {
  const data = JSON.parse(ev.data);
  if (!validate(data)) {
    console.warn("contract drift", validate.errors, data);
  }
  // ...
});
```
