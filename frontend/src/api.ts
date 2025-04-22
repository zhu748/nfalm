export async function getVersion() {
  const response = await fetch("/api/version");
  return await response.text();
}

/**
 * Sends a cookie to the server.
 * @param cookie The cookie string to send
 * @returns The fetch response object
 *
 * Possible Status Codes:
 * - 200: Success
 * - 400: Invalid cookie
 * - 401: Invalid bearer token
 * - 500: Server error
 */
export async function postCookie(cookie: string) {
  const bearer = localStorage.getItem("authToken") || "";
  if (!bearer) {
    throw new Error("Bearer token is missing");
  }
  const response = await fetch("/api/submit", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${bearer}`,
    },
    body: JSON.stringify({ cookie }),
  });

  return response;
}
