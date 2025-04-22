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
  const response = await fetch("/api/submit", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${bearer}`,
    },
    body: JSON.stringify({ cookie }),
  });

  if (response.status === 400) {
    throw new Error("Invalid cookie format");
  } else if (response.status === 401) {
    throw new Error("Authentication failed. Please set a valid auth token.");
  } else if (response.status === 500) {
    throw new Error("Server error.");
  }

  if (!response.ok) {
    throw new Error(`Error ${response.status}: ${response.statusText}`);
  }
}

/**
 * Gets cookie status information from the server.
 * @returns The cookie status data
 *
 * Possible Status Codes:
 * - 200: Success with cookie status data
 * - 401: Invalid bearer token
 * - 500: Server error
 */
export async function getCookieStatus() {
  const bearer = localStorage.getItem("authToken") || "";
  const response = await fetch("/api/get_cookies", {
    method: "GET",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${bearer}`,
    },
  });

  if (!response.ok) {
    throw new Error(`Error ${response.status}: ${response.statusText}`);
  }

  return await response.json();
}
