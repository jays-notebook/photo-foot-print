import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, waitFor } from "@testing-library/svelte";
import DetailPane from "./DetailPane.svelte";
import * as ipc from "../ipc";
import * as dialog from "../save/saveDialog";

vi.mock("../ipc", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../ipc")>()),
  readPhotoMeta: vi.fn(), saveGeotag: vi.fn(), getSessionLastPin: vi.fn(),
}));
vi.mock("../save/saveDialog", () => ({
  confirmSaveGeotag: vi.fn(), showSaveError: vi.fn(),
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => { resolve = r; });
  return { promise, resolve };
}
const summary = (id: string): ipc.PhotoSummary => ({
  id, file_name: `${id}.jpg`, has_gps: false, capture_time: null,
  size_bytes: 10, mtime_unix: 0,
});
const meta = (id: string): ipc.PhotoMeta => ({
  id, file_name: `${id}.jpg`, gps: null, capture_time: null,
  altitude_m: null, dimensions: null,
});
beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(ipc.getSessionLastPin).mockResolvedValue(null);
  vi.mocked(dialog.confirmSaveGeotag).mockResolvedValue(true);
});
afterEach(cleanup);

test("disables saving until the selected photo has loaded", async () => {
  const read = deferred<ipc.PhotoMeta>();
  vi.mocked(ipc.readPhotoMeta).mockResolvedValueOnce(meta("A")).mockReturnValueOnce(read.promise);
  const view = render(DetailPane, { summary: summary("A") });
  const save = () => view.getByRole("button", { name: "Save" }) as HTMLButtonElement;
  await waitFor(() => expect(save().disabled).toBe(false));
  await view.rerender({ summary: summary("B") });
  await waitFor(() => expect(save().disabled).toBe(true));
  await fireEvent.click(save());
  expect(dialog.confirmSaveGeotag).not.toHaveBeenCalled();
  read.resolve(meta("B"));
  await waitFor(() => expect(save().disabled).toBe(false));
});

test("ignores an old save response in the new editor but updates its list row", async () => {
  const write = deferred<ipc.PhotoMeta>();
  const onSaved = vi.fn();
  vi.mocked(ipc.readPhotoMeta).mockResolvedValueOnce(meta("A")).mockResolvedValueOnce({
    ...meta("B"), capture_time: "2000:01:01 00:00:00",
  });
  vi.mocked(ipc.saveGeotag).mockReturnValueOnce(write.promise);
  const view = render(DetailPane, { summary: summary("A"), onSaved });
  await waitFor(() => expect((view.getByRole("button", {name: "Save"}) as HTMLButtonElement).disabled).toBe(false));
  await fireEvent.click(view.getByRole("button", {name: "Save"}));
  await waitFor(() => expect(ipc.saveGeotag).toHaveBeenCalled());
  await view.rerender({ summary: summary("B"), onSaved });
  const input = () => view.getByLabelText("Capture time") as HTMLInputElement;
  await waitFor(() => expect(input().value).toBe("2000-01-01T00:00"));
  const fresh = { ...meta("A"), gps: {lat: 37, lng: 127}, capture_time: "2024:02:03 04:05:00" };
  write.resolve(fresh);
  await waitFor(() => expect(onSaved).toHaveBeenCalledWith(fresh));
  expect(input().value).toBe("2000-01-01T00:00");
});

test("cancels a pending confirmation when selection changes", async () => {
  const confirm = deferred<boolean>();
  vi.mocked(ipc.readPhotoMeta).mockImplementation(async (id) => meta(id));
  vi.mocked(dialog.confirmSaveGeotag).mockReturnValue(confirm.promise);
  const view = render(DetailPane, { summary: summary("A") });
  await waitFor(() => expect((view.getByRole("button", {name: "Save"}) as HTMLButtonElement).disabled).toBe(false));
  await fireEvent.click(view.getByRole("button", {name: "Save"}));
  await waitFor(() => expect(dialog.confirmSaveGeotag).toHaveBeenCalledTimes(1));
  await view.rerender({ summary: summary("B") });
  confirm.resolve(true);
  await waitFor(() => expect(view.queryByText("Saving...")).toBeNull());
  expect(ipc.saveGeotag).not.toHaveBeenCalled();
});


test("GPS-only save leaves capture and digitized timestamps untouched", async () => {
  const saved = { ...meta("A"), capture_time: "1990:01:01 00:00:00" };
  vi.mocked(ipc.readPhotoMeta).mockResolvedValue(saved);
  vi.mocked(ipc.saveGeotag).mockResolvedValue(saved);
  const view = render(DetailPane, { summary: summary("A") });
  await waitFor(() => expect((view.getByRole("button", {name: "Save"}) as HTMLButtonElement).disabled).toBe(false));
  await fireEvent.click(view.getByRole("button", {name: "Save"}));
  await waitFor(() => expect(ipc.saveGeotag).toHaveBeenCalledWith("A", 37.5665, 126.978, {kind: "keep"}));
  expect(dialog.confirmSaveGeotag).toHaveBeenCalledWith(expect.objectContaining({dirtyDto: false}));
});


test.each([
  ["", {kind: "remove"}],
  ["2001-02-03T04:05", {kind: "set", value: "2001:02:03 04:05:00"}],
])("sends an explicit capture-time operation for %s", async (value, operation) => {
  const saved = { ...meta("A"), gps: {lat: 37, lng: 127}, capture_time: "1990:01:01 00:00:00" };
  vi.mocked(ipc.readPhotoMeta).mockResolvedValue(saved);
  vi.mocked(ipc.saveGeotag).mockResolvedValue({...saved, capture_time: null});
  const view = render(DetailPane, { summary: summary("A") });
  await waitFor(() => expect(view.getByLabelText("Capture time")).toBeTruthy());
  await fireEvent.input(view.getByLabelText("Capture time"), {target: {value}});
  await fireEvent.click(view.getByRole("button", {name: "Save"}));
  await waitFor(() => expect(ipc.saveGeotag).toHaveBeenCalledWith("A", 37, 127, operation));
});
