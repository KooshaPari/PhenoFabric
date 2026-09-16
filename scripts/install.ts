# terminal-fabric installer for Windows (bun/deno style)
# Usage: bun run https://raw.githubusercontent.com/<REDACTED>/terminal-fabric/main/scripts/install.ts
const REPO = "<REDACTED>/terminal-fabric";
const BINARY = "tf-win-capture";

async function install() {
  console.log(`Installing ${BINARY}...`);

  // Get latest release
  const resp = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`);
  const release = await resp.json();
  const version = release.tag_name.replace(/^v/, "");

  const asset = release.assets.find((a: any) => a.name.includes("tf-win-capture.exe"));
  if (!asset) throw new Error("No tf-win-capture.exe found in latest release");

  console.log(`Downloading ${BINARY} v${version}...`);

  const installDir = `${Deno.env.get("USERPROFILE") || Deno.env.get("HOME")}/.tf`;
  await Deno.mkdir(installDir, { recursive: true });

  const exePath = `${installDir}/${BINARY}.exe`;

  // Download
  const dlResp = await fetch(asset.browser_download_url);
  const bytes = new Uint8Array(await dlResp.arrayBuffer());
  await Deno.writeFile(exePath, bytes);

  console.log(`Installed ${BINARY} v${version} to ${exePath}`);
}

install().catch(console.error);