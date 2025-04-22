export async function getVersion() {
  const response = await fetch("/api/version");
  if (!response.ok) {
    throw new Error("Network response was not ok");
  }
  return await response.text();
}
