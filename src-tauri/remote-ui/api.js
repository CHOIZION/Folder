export function createApiClient(token) {
  return async function api(path, options = {}) {
    const headers = {
      ...(options.headers || {}),
      Authorization: `Bearer ${token}`,
    };
    if (options.body && !headers["Content-Type"]) {
      headers["Content-Type"] = "application/json";
    }

    const response = await fetch(path, {
      ...options,
      headers,
      cache: "no-store",
    });
    if (response.ok) return response;

    let message = "요청에 실패했습니다.";
    try {
      message = (await response.json()).message || message;
    } catch {
      // The server can close a stream before returning a JSON error body.
    }
    const error = new Error(message);
    error.status = response.status;
    throw error;
  };
}
