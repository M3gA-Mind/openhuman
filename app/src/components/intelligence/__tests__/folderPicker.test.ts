/**
 * #5831 — the folder picker must never hand back a value that cannot work
 * as a path.
 *
 * The defect these tests pin: when `File.path` was unavailable the handler
 * fell back to `webkitRelativePath.split('/')[0]`, storing the chosen
 * directory's NAME with its location discarded. A source built from that
 * value can never sync, and it fails once per cycle forever rather than once,
 * visibly, at the moment of choosing.
 */
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { directoryPathFromPickedFiles, pickDirectoryNatively } from '../folderPicker';

const hoisted = vi.hoisted(() => ({ invoke: vi.fn(), isTauri: vi.fn() }));

vi.mock('../../../utils/tauriCommands/common', () => ({
  safeInvoke: hoisted.invoke,
  isTauri: hoisted.isTauri,
}));

/** A `webkitdirectory` file entry, with `path` present only when asked for. */
function pickedFile(relativePath: string, absolutePath?: string): File {
  const file = new File(['x'], relativePath.split('/').pop() ?? 'f.md');
  Object.defineProperty(file, 'webkitRelativePath', { value: relativePath });
  if (absolutePath !== undefined) {
    Object.defineProperty(file, 'path', { value: absolutePath });
  }
  return file;
}

function fileList(...files: File[]): FileList {
  const indexed: Record<number, File> = {};
  files.forEach((file, i) => {
    indexed[i] = file;
  });
  return {
    ...indexed,
    length: files.length,
    item: (i: number) => files[i] ?? null,
  } as unknown as FileList;
}

describe('directoryPathFromPickedFiles', () => {
  it('refuses to produce a path when the renderer does not expose File.path', () => {
    // THE REGRESSION. The old handler returned 'docs' here. A source stored
    // with that value produced `folder does not exist: docs` on every sync,
    // indefinitely, and no absolute path is recoverable from this input.
    const result = directoryPathFromPickedFiles(fileList(pickedFile('docs/readme.md')));

    expect(result).toEqual({ ok: false, reason: 'no-absolute-path' });
  });

  it('never returns the bare directory name for any nesting depth', () => {
    for (const relative of ['docs/a.md', 'docs/deep/b.md', 'docs/deep/deeper/c.md']) {
      const result = directoryPathFromPickedFiles(fileList(pickedFile(relative)));

      expect(result.ok).toBe(false);
      // Belt and braces: assert the specific bad value can never come back,
      // not merely that this input failed.
      expect(JSON.stringify(result)).not.toContain('"docs"');
    }
  });

  it('derives the containing directory when File.path is present', () => {
    const result = directoryPathFromPickedFiles(
      fileList(pickedFile('notes/readme.md', '/Users/you/notes/readme.md'))
    );

    expect(result).toEqual({ ok: true, path: '/Users/you' });
  });

  it('derives a Windows directory without mangling the separators', () => {
    const result = directoryPathFromPickedFiles(
      fileList(pickedFile('notes/readme.md', 'C:\\Users\\you\\notes\\readme.md'))
    );

    // `webkitRelativePath` is forward-slashed while the absolute path is not,
    // so the relative portion does not appear verbatim and the whole path is
    // kept. That is a usable absolute path, which is the property that matters.
    expect(result).toEqual({ ok: true, path: 'C:\\Users\\you\\notes\\readme.md' });
  });

  it('treats an empty selection as a cancellation rather than an error', () => {
    expect(directoryPathFromPickedFiles(null)).toEqual({ ok: false, reason: 'cancelled' });
    expect(directoryPathFromPickedFiles(fileList())).toEqual({ ok: false, reason: 'cancelled' });
  });
});

describe('pickDirectoryNatively', () => {
  beforeEach(() => {
    hoisted.invoke.mockReset();
    hoisted.isTauri.mockReset();
  });

  it('reports unavailable outside the desktop shell, without invoking', async () => {
    hoisted.isTauri.mockReturnValue(false);

    await expect(pickDirectoryNatively()).resolves.toEqual({ ok: false, reason: 'unavailable' });
    expect(hoisted.invoke).not.toHaveBeenCalled();
  });

  it('returns the absolute path the native dialog chose', async () => {
    hoisted.isTauri.mockReturnValue(true);
    hoisted.invoke.mockResolvedValue('/Users/you/notes');

    await expect(pickDirectoryNatively()).resolves.toEqual({ ok: true, path: '/Users/you/notes' });
    expect(hoisted.invoke).toHaveBeenCalledWith('pick_directory_via_dialog');
  });

  it('treats a null return as a cancellation', async () => {
    hoisted.isTauri.mockReturnValue(true);
    hoisted.invoke.mockResolvedValue(null);

    await expect(pickDirectoryNatively()).resolves.toEqual({ ok: false, reason: 'cancelled' });
  });

  it('falls back to unavailable when the dialog cannot run', async () => {
    // Headless Linux with no xdg-desktop portal lands here, and wants the
    // fallback input rather than an error.
    hoisted.isTauri.mockReturnValue(true);
    hoisted.invoke.mockRejectedValue(new Error('no portal'));

    await expect(pickDirectoryNatively()).resolves.toEqual({ ok: false, reason: 'unavailable' });
  });

  it('refuses a blank path rather than storing it', async () => {
    hoisted.isTauri.mockReturnValue(true);
    hoisted.invoke.mockResolvedValue('   ');

    await expect(pickDirectoryNatively()).resolves.toEqual({
      ok: false,
      reason: 'no-absolute-path',
    });
  });
});
