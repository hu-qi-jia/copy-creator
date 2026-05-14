export async function withOpacityHidden(action: () => Promise<void>): Promise<void> {
  try {
    await action();
  } catch (e) {
    console.error("Paste failed:", e);
  }
}
