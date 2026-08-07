# Privacy and threat model

TraceForge is a static application. Imported bytes are passed directly to WebAssembly memory and are never sent through `fetch`, XHR, WebSocket, a service worker upload or analytics SDK. There are no accounts, cookies or server-side logs owned by TraceForge.

The browser boundary rejects files over 50 MB and datasets over 100,000 valid events. Invalid raw rows are truncated to 240 characters in reports. The included service worker caches only GET resources required by the application.

Synthetic fixtures use documentation address ranges and invented identities. Do not publish screenshots containing real incident data. Browser extensions, the hosting provider and the local device remain outside this application's trust boundary.

