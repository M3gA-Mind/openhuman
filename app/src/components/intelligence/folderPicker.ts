/**
 * Folder selection for the folder memory-source (#5831).
 *
 * The rule this module exists to enforce: **never hand back a value that
 * cannot work as a path.** A folder source is stored verbatim and read back
 * by the memory driver, so a value that is not an absolute path produces a
 * source that looks configured and can never sync — failing once per cycle,
 * indefinitely, with an error the user cannot connect to the picker they
 * used. Refusing at the point of choosing is strictly better: it is one
 * failure, immediately, next to the control that caused it.
 *
 * Two selection paths, in preference order:
 *
 * 1. {@link pickDirectoryNatively} — the OS directory chooser, via the
 *    `pick_directory_via_dialog` Tauri command. Returns an absolute path in
 *    every renderer and on every platform. This is the durable answer.
 * 2. {@link directoryPathFromPickedFiles} — the `<input webkitdirectory>`
 *    fallback for a browser context, where no native dialog exists. It can
 *    only produce a real path when Chromium exposes the non-standard
 *    `File.path`; when it does not, this returns a failure rather than the
 *    bare directory name the old code stored.
 */
import { safeInvoke as invoke, isTauri } from '../../utils/tauriCommands/common';

/**
 * Why a folder selection produced no usable path.
 *
 * - `cancelled` — the user dismissed the chooser. Not an error; the caller
 *   leaves the field as it was and says nothing.
 * - `unavailable` — no native chooser here (a browser context, or the
 *   dialog could not run). The caller offers the fallback input.
 * - `no-absolute-path` — a directory was chosen but the renderer would not
 *   say where it is. **This is the case that must never be stored.**
 */
export type FolderPickFailure = 'cancelled' | 'unavailable' | 'no-absolute-path';

export type FolderPickResult =
  | { ok: true; path: string }
  | { ok: false; reason: FolderPickFailure };

/**
 * Derive the chosen directory's absolute path from a `webkitdirectory`
 * `FileList`.
 *
 * Chromium exposes the absolute path of each file on the non-standard
 * `File.path` attribute only when the renderer has filesystem-aware
 * integration. When present, the directory is that path with the file's
 * `webkitRelativePath` (`<dir>/<...>/<file>`) trimmed off the end.
 *
 * When it is absent, all that remains is `webkitRelativePath`, whose first
 * segment is the directory's **name** — not its location. The old handler
 * stored that name and it could never resolve, which is #5831. So this
 * returns `no-absolute-path` instead: the caller must surface it, not save it.
 */
export function directoryPathFromPickedFiles(files: FileList | null): FolderPickResult {
  if (!files || files.length === 0) {
    return { ok: false, reason: 'cancelled' };
  }

  const first = files[0] as File & { path?: string };
  const absolute = first.path;

  if (!absolute) {
    // `webkitRelativePath` is deliberately NOT consulted as a fallback. Its
    // first segment is a name with the location discarded; storing it is the
    // defect, not a degraded-but-usable answer.
    return { ok: false, reason: 'no-absolute-path' };
  }

  const relative = first.webkitRelativePath || first.name;
  const cut = absolute.lastIndexOf(relative);
  // `cut > 0` keeps a pathological `lastIndexOf` result (0, or -1 when the
  // relative portion is somehow absent) from truncating the path to nothing.
  const directory = cut > 0 ? absolute.slice(0, cut) : absolute;
  const trimmed = directory.replace(/[/\\]+$/, '');

  // A trailing-separator trim can empty a root-level selection ("/" -> ""),
  // and an empty path is exactly the unusable value this module refuses.
  if (trimmed.length === 0) {
    return { ok: true, path: directory };
  }
  return { ok: true, path: trimmed };
}

/**
 * Open the OS-native directory chooser.
 *
 * Resolves `unavailable` outside the desktop shell, and also when the
 * command throws — a headless Linux host with no xdg-desktop portal reaches
 * the same place as a browser tab, and both want the fallback input rather
 * than an error.
 */
export async function pickDirectoryNatively(): Promise<FolderPickResult> {
  if (!isTauri()) {
    return { ok: false, reason: 'unavailable' };
  }

  try {
    const chosen = await invoke<string | null>('pick_directory_via_dialog');
    if (chosen == null) {
      return { ok: false, reason: 'cancelled' };
    }
    if (chosen.trim().length === 0) {
      return { ok: false, reason: 'no-absolute-path' };
    }
    return { ok: true, path: chosen };
  } catch {
    return { ok: false, reason: 'unavailable' };
  }
}
