import { useEffect, useRef, useState } from "react";
import { useDebouncedValue } from "./use-debounced-value";

/**
 * Local <Input> state for a debounced text filter (search box, action box,
 * etc.) pushed into `useServerTable`'s filters. Seeded from the table's
 * already-URL-restored value and skips pushing on the first render, so
 * navigating back to a page with a search filter set doesn't get clobbered
 * by the debounce effect re-pushing an empty string over the restored one.
 */
export function useSearchFilterSync(
  initialValue: string,
  onDebouncedChange: (value: string) => void,
  delayMs = 250
) {
  const [input, setInput] = useState(initialValue);
  const debounced = useDebouncedValue(input, delayMs);
  const isFirstRun = useRef(true);

  useEffect(() => {
    if (isFirstRun.current) {
      isFirstRun.current = false;
      return;
    }
    onDebouncedChange(debounced);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [debounced]);

  return [input, setInput] as const;
}
