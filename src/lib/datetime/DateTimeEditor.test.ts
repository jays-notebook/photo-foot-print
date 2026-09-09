import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { afterEach, expect, test, vi } from "vitest";
import DateTimeEditor from "./DateTimeEditor.svelte";

afterEach(cleanup);
test("does not turn a partial date edit into a deletion", async () => {
  const onChange = vi.fn();
  const onValidityChange = vi.fn();
  const view = render(DateTimeEditor, {value: "1990:01:01 00:00:00", onChange, onValidityChange});
  const input = view.getByLabelText("Capture time") as HTMLInputElement;
  Object.defineProperty(input, "validity", {value: {valid: false, badInput: true}});
  await fireEvent.input(input, {target: {value: ""}});
  expect(onChange).not.toHaveBeenCalled();
  expect(onValidityChange).toHaveBeenCalledWith(false);
});
