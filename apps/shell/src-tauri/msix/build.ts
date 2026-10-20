// Builds and signs the MSIX sparse package.
//
// The package exists only to give the installed `bw.exe` package identity, so
// that `UserNotificationListener` will report what other applications have
// posted to the Action Center. Everything else in the shell works without it.
//
// This is separate from `tauri build` on purpose. Signing needs a certificate,
// and a certificate is a decision about the machine — which one to trust, and
// who is allowed to install one — that no build script should make on somebody
// else's behalf. So this takes a certificate you already have and does not
// create, install or trust anything.
//
// See docs/msix.md for what to do with the result, including how to make a
// certificate for testing and what trusting one actually means.
//
//     node build.ts --certificate .\mine.pfx [--password <pfx password>] [--output <file.msix>]
//
// --certificate  the .pfx to sign with. Its subject is written into the
//                manifest's Publisher, which Windows requires to match exactly.
// --password     the .pfx password, if it has one.
// --output       where to write the .msix; beside this script by default.

import { execFileSync } from "node:child_process";
import { X509Certificate } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const here = path.dirname(fileURLToPath(import.meta.url));

const { values } = parseArgs({
  options: {
    certificate: { type: "string" },
    password: { type: "string" },
    output: {
      type: "string",
      default: path.join(here, "beautiful-wallpaper-sparse.msix"),
    },
  },
});
if (!values.certificate) {
  console.error(
    "usage: node build.ts --certificate <file.pfx> [--password <password>] [--output <file.msix>]",
  );
  process.exit(1);
}
const certificate = path.resolve(values.certificate);
const password = values.password ?? "";
const output = path.resolve(values.output);

/**
 * makeappx and signtool come with the Windows SDK and are not on PATH by
 * default. Looking them up beats telling somebody to "add the SDK to PATH",
 * which is the step people get wrong.
 */
function findSdkTool(name: string): string {
  try {
    const onPath = execFileSync("where", [name], { encoding: "utf8" })
      .split(/\r?\n/)
      .find(Boolean);
    if (onPath) return onPath;
  } catch {
    // Not on PATH; look in the SDK.
  }

  const found: string[] = [];
  const walk = (folder: string) => {
    for (const entry of readdirSync(folder, { withFileTypes: true })) {
      const full = path.join(folder, entry.name);
      if (entry.isDirectory()) walk(full);
      else if (entry.name.toLowerCase() === name && /\\x64\\/i.test(full)) {
        found.push(full);
      }
    }
  };
  for (const root of [
    process.env["ProgramFiles(x86)"],
    process.env["ProgramFiles"],
  ]) {
    const bin = root && path.join(root, "Windows Kits", "10", "bin");
    if (bin && existsSync(bin)) walk(bin);
  }
  const newest = found.sort().at(-1);
  if (!newest) {
    throw new Error(
      `${name} was not found. It comes with the Windows SDK; install it, or run this from a Developer Command Prompt.`,
    );
  }
  return newest;
}

/**
 * The certificate's subject as Windows writes it — the form the manifest's
 * Publisher has to match exactly. A mismatch here is the single most common
 * reason Windows refuses a package, and the error it gives says nothing useful
 * about which of the two is wrong.
 *
 * Node cannot open a .pfx, so `certutil` splits the certificates out of it,
 * and Node reads the one that is not a CA. Windows lists the name's parts in
 * the reverse of their order in the certificate, joined by ", ", which is what
 * `X509Certificate2.Subject` and `certutil -dump` show.
 */
function subjectOf(pfx: string, pass: string, scratch: string): string {
  const folder = path.join(scratch, "split");
  mkdirSync(folder);
  execFileSync("certutil", ["-p", pass, "-split", "-dump", pfx], {
    cwd: folder,
    stdio: "ignore",
  });
  const certificates = readdirSync(folder)
    .filter((file) => file.toLowerCase().endsWith(".crt"))
    .map((file) => new X509Certificate(readFileSync(path.join(folder, file))));
  const leaf =
    certificates.find((certificate) => !certificate.ca) ?? certificates[0];
  if (!leaf) throw new Error(`no certificate found in ${pfx}`);
  // ponytail: quotes a value with a special character the way Windows does
  // for the common cases only; a multi-valued part ("+") is passed through.
  return leaf.subject
    .split("\n")
    .reverse()
    .map((part) => {
      const [key, ...rest] = part.split("=");
      const value = rest.join("=").replace(/\\(.)/g, "$1");
      return /[,+=";<>#\n]|^\s|\s$/.test(value)
        ? `${key}="${value.replace(/"/g, '""')}"`
        : `${key}=${value}`;
    })
    .join(", ");
}

const makeappx = findSdkTool("makeappx.exe");
const signtool = findSdkTool("signtool.exe");

const staging = mkdtempSync(path.join(tmpdir(), "bw-sparse-"));
try {
  const subject = subjectOf(certificate, password, staging);
  console.log(`Signing as: ${subject}`);

  const packageRoot = path.join(staging, "package");
  mkdirSync(packageRoot);
  const manifest = readFileSync(path.join(here, "AppxManifest.xml"), "utf8")
    // The subject has quotes of its own when a part needs them; the
    // attribute's are written as entities so the two do not collide.
    .replace(
      /Publisher="[^"]*"/,
      `Publisher="${subject.replace(/&/g, "&amp;").replace(/"/g, "&quot;")}"`,
    );
  writeFileSync(path.join(packageRoot, "AppxManifest.xml"), manifest);

  // The logos the manifest names. A package whose manifest points at an image
  // that is not in it fails to pack, with a message about the image rather
  // than about the manifest.
  const images = path.join(packageRoot, "images");
  mkdirSync(images);
  const icons = path.join(here, "..", "icons");
  copyFileSync(
    path.join(icons, "128x128.png"),
    path.join(images, "StoreLogo.png"),
  );
  copyFileSync(
    path.join(icons, "128x128.png"),
    path.join(images, "Square150x150Logo.png"),
  );
  copyFileSync(
    path.join(icons, "32x32.png"),
    path.join(images, "Square44x44Logo.png"),
  );

  // `/nv` skips validation that only applies to packages carrying payload.
  execFileSync(
    makeappx,
    ["pack", "/d", packageRoot, "/p", output, "/nv", "/o"],
    { stdio: "inherit" },
  );
  execFileSync(
    signtool,
    [
      "sign",
      "/fd",
      "SHA256",
      "/a",
      "/f",
      certificate,
      ...(password ? ["/p", password] : []),
      output,
    ],
    { stdio: "inherit" },
  );

  console.log(`Wrote ${output}`);
  console.log(
    "Put it beside bw.exe in the install folder, then switch on hacks.readOtherNotifications and restart the shell.",
  );
} finally {
  rmSync(staging, { recursive: true, force: true });
}
