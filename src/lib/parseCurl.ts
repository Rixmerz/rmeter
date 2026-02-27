/**
 * Parses a cURL command string into structured request components.
 * Handles common curl flags: -X, -H, -d/--data, --data-raw, --data-urlencode,
 * -u/--user, -A/--user-agent, --compressed, -k/--insecure, -L/--location, etc.
 */

export interface ParsedCurl {
  method: string;
  url: string;
  headers: Record<string, string>;
  body: string | null;
  bodyType: "none" | "json" | "form_data" | "raw" | "xml";
  formData: [string, string][] | null;
  auth: { type: "basic"; username: string; password: string } | { type: "bearer"; token: string } | null;
}

/**
 * Tokenize a curl command string, handling quoted strings and backslash escapes.
 */
function tokenize(input: string): string[] {
  const tokens: string[] = [];
  let current = "";
  let inSingle = false;
  let inDouble = false;
  let escape = false;

  // Normalize line continuations (backslash + newline)
  const normalized = input.replace(/\\\s*\n\s*/g, " ").trim();

  for (let i = 0; i < normalized.length; i++) {
    const ch = normalized[i];

    if (escape) {
      current += ch;
      escape = false;
      continue;
    }

    if (ch === "\\") {
      if (inSingle) {
        current += ch;
      } else {
        escape = true;
      }
      continue;
    }

    if (ch === "'" && !inDouble) {
      inSingle = !inSingle;
      continue;
    }

    if (ch === '"' && !inSingle) {
      inDouble = !inDouble;
      continue;
    }

    if ((ch === " " || ch === "\t") && !inSingle && !inDouble) {
      if (current.length > 0) {
        tokens.push(current);
        current = "";
      }
      continue;
    }

    current += ch;
  }

  if (current.length > 0) {
    tokens.push(current);
  }

  return tokens;
}

function detectBodyType(body: string, headers: Record<string, string>): "json" | "xml" | "raw" {
  const contentType = Object.entries(headers).find(
    ([k]) => k.toLowerCase() === "content-type"
  )?.[1]?.toLowerCase() ?? "";

  if (contentType.includes("application/json")) return "json";
  if (contentType.includes("xml")) return "xml";

  // Auto-detect from body content
  const trimmed = body.trim();
  if ((trimmed.startsWith("{") && trimmed.endsWith("}")) ||
      (trimmed.startsWith("[") && trimmed.endsWith("]"))) {
    try {
      JSON.parse(trimmed);
      return "json";
    } catch { /* not json */ }
  }
  if (trimmed.startsWith("<") && trimmed.endsWith(">")) return "xml";

  return "raw";
}

export function parseCurl(input: string): ParsedCurl {
  const tokens = tokenize(input);

  // Skip leading "curl" if present
  let start = 0;
  if (tokens[0]?.toLowerCase() === "curl") {
    start = 1;
  }

  let method: string | null = null;
  let url = "";
  const headers: Record<string, string> = {};
  let body: string | null = null;
  let auth: ParsedCurl["auth"] = null;
  const formDataParts: [string, string][] = [];
  let isFormData = false;

  for (let i = start; i < tokens.length; i++) {
    const token = tokens[i];

    // Method
    if (token === "-X" || token === "--request") {
      method = tokens[++i]?.toUpperCase() ?? "GET";
      continue;
    }

    // Headers
    if (token === "-H" || token === "--header") {
      const headerStr = tokens[++i] ?? "";
      const colonIdx = headerStr.indexOf(":");
      if (colonIdx > 0) {
        const key = headerStr.slice(0, colonIdx).trim();
        const value = headerStr.slice(colonIdx + 1).trim();
        headers[key] = value;
      }
      continue;
    }

    // Data (body)
    if (token === "-d" || token === "--data" || token === "--data-raw" || token === "--data-binary") {
      body = tokens[++i] ?? "";
      continue;
    }

    // URL-encoded data
    if (token === "--data-urlencode") {
      const part = tokens[++i] ?? "";
      const eqIdx = part.indexOf("=");
      if (eqIdx > 0) {
        formDataParts.push([part.slice(0, eqIdx), part.slice(eqIdx + 1)]);
      } else {
        formDataParts.push([part, ""]);
      }
      isFormData = true;
      continue;
    }

    // Form data (-F/--form)
    if (token === "-F" || token === "--form") {
      const part = tokens[++i] ?? "";
      const eqIdx = part.indexOf("=");
      if (eqIdx > 0) {
        formDataParts.push([part.slice(0, eqIdx), part.slice(eqIdx + 1)]);
      } else {
        formDataParts.push([part, ""]);
      }
      isFormData = true;
      continue;
    }

    // Basic auth
    if (token === "-u" || token === "--user") {
      const userPass = tokens[++i] ?? "";
      const colonIdx = userPass.indexOf(":");
      if (colonIdx > 0) {
        auth = {
          type: "basic",
          username: userPass.slice(0, colonIdx),
          password: userPass.slice(colonIdx + 1),
        };
      } else {
        auth = { type: "basic", username: userPass, password: "" };
      }
      continue;
    }

    // User-Agent
    if (token === "-A" || token === "--user-agent") {
      headers["User-Agent"] = tokens[++i] ?? "";
      continue;
    }

    // Flags that take no argument — skip
    if (token === "--compressed" || token === "-k" || token === "--insecure" ||
        token === "-L" || token === "--location" || token === "-s" || token === "--silent" ||
        token === "-S" || token === "--show-error" || token === "-v" || token === "--verbose" ||
        token === "-i" || token === "--include" || token === "-I" || token === "--head" ||
        token === "-g" || token === "--globoff") {
      if (token === "-I" || token === "--head") {
        method = method ?? "HEAD";
      }
      continue;
    }

    // Output file (skip the argument)
    if (token === "-o" || token === "--output" || token === "-b" || token === "--cookie" ||
        token === "-c" || token === "--cookie-jar" || token === "--connect-timeout" ||
        token === "-m" || token === "--max-time" || token === "--retry" ||
        token === "-w" || token === "--write-out" || token === "--cacert" ||
        token === "--cert" || token === "--key" || token === "-e" || token === "--referer") {
      i++; // skip the argument
      continue;
    }

    // If it looks like a URL (not a flag), treat it as the URL
    if (!token.startsWith("-") && !url) {
      url = token;
      continue;
    }
  }

  // Check for Bearer token in Authorization header
  const authHeader = Object.entries(headers).find(
    ([k]) => k.toLowerCase() === "authorization"
  );
  if (authHeader && !auth) {
    const val = authHeader[1];
    if (val.toLowerCase().startsWith("bearer ")) {
      auth = { type: "bearer", token: val.slice(7).trim() };
      delete headers[authHeader[0]];
    } else if (val.toLowerCase().startsWith("basic ")) {
      try {
        const decoded = atob(val.slice(6).trim());
        const colonIdx = decoded.indexOf(":");
        if (colonIdx > 0) {
          auth = { type: "basic", username: decoded.slice(0, colonIdx), password: decoded.slice(colonIdx + 1) };
        }
      } catch { /* leave as header */ }
      delete headers[authHeader[0]];
    }
  }

  // Determine body type
  let bodyType: ParsedCurl["bodyType"] = "none";
  let formData: [string, string][] | null = null;

  if (isFormData && formDataParts.length > 0) {
    bodyType = "form_data";
    formData = formDataParts;
    body = null;
  } else if (body !== null) {
    bodyType = detectBodyType(body, headers);
  }

  // Default method
  if (!method) {
    method = body !== null || isFormData ? "POST" : "GET";
  }

  return { method, url, headers, body, bodyType, formData, auth };
}
