pub fn inject_csrf_token(html: &str, csrf_token: &str) -> String {
    if let Some(head_end) = html.find("</head>") {
        let mut result = String::with_capacity(html.len() + 100);

        result.push_str(&html[..head_end]);

        result.push_str(&format!(
            r#"    <meta name="csrf-token" content="{}" />
"#,
            csrf_token
        ));

        result.push_str(&html[head_end..]);

        result
    } else {
        html.to_string()
    }
}

pub fn generate_csrf_helper_script() -> &'static str {
    r#"<script>
(function() {
  window.getCsrfToken = function() {
    const meta = document.querySelector('meta[name="csrf-token"]');
    return meta ? meta.content : null;
  };

  window.fetchWithCsrf = async function(url, options = {}) {
    const token = window.getCsrfToken();

    if (!token) {
      console.warn('CSRF token not found, attempting to refresh...');
      await window.refreshCsrfToken();
    }

    const headers = options.headers || {};
    const finalToken = token || window.getCsrfToken();
    if (finalToken) {
      headers['X-CSRF-Token'] = finalToken;
    }

    const response = await fetch(url, {
      ...options,
      headers
    });

    if (response.status === 403 && url.includes('/_rari/')) {
      const refreshed = await window.refreshCsrfToken();
      if (refreshed) {
        const retryToken = window.getCsrfToken();
        if (retryToken) {
          headers['X-CSRF-Token'] = retryToken;
          return fetch(url, {
            ...options,
            headers
          });
        }
      }
    }

    return response;
  };

  window.injectCsrfIntoForms = function() {
    const token = window.getCsrfToken();
    if (!token) return;

    document.querySelectorAll('form[action*="/_rari/"]').forEach(form => {
      let csrfInput = form.querySelector('input[name="__csrf_token"]');

      if (!csrfInput) {
        csrfInput = document.createElement('input');
        csrfInput.type = 'hidden';
        csrfInput.name = '__csrf_token';
        form.appendChild(csrfInput);
      }

      csrfInput.value = token;
    });
  };

  window.refreshCsrfToken = async function() {
    try {
      const response = await fetch('/_rari/csrf-token');
      if (!response.ok) {
        console.warn('Failed to refresh CSRF token:', response.status);
        return false;
      }
      const data = await response.json();
      if (data.token) {
        let meta = document.querySelector('meta[name="csrf-token"]');
        if (!meta) {
          meta = document.createElement('meta');
          meta.name = 'csrf-token';
          document.head.appendChild(meta);
        }
        meta.content = data.token;

        window.injectCsrfIntoForms();
        return true;
      }
      return false;
    } catch (error) {
      console.error('Error refreshing CSRF token:', error);
      return false;
    }
  };

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', window.injectCsrfIntoForms);
  } else {
    window.injectCsrfIntoForms();
  }

  if (typeof MutationObserver !== 'undefined') {
    const observer = new MutationObserver(function(mutations) {
      mutations.forEach(function(mutation) {
        if (mutation.addedNodes.length) {
          window.injectCsrfIntoForms();
        }
      });
    });

    observer.observe(document.body, {
      childList: true,
      subtree: true
    });
  }

  window.addEventListener('rari:navigate', async function() {
    await window.refreshCsrfToken();
  });
})();
</script>"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_csrf_token_into_html() {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Test</title>
</head>
<body>
    <h1>Hello</h1>
</body>
</html>"#;

        let token = "test-token-123";
        let result = inject_csrf_token(html, token);

        assert!(result.contains(r#"<meta name="csrf-token" content="test-token-123" />"#));
        assert!(result.contains("<title>Test</title>"));
        assert!(result.contains("<h1>Hello</h1>"));
    }

    #[test]
    fn test_inject_csrf_token_preserves_structure() {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
</head>
<body></body>
</html>"#;

        let token = "abc123";
        let result = inject_csrf_token(html, token);

        let head_end = result.find("</head>").unwrap();
        let token_pos = result.find(r#"<meta name="csrf-token""#).unwrap();
        assert!(token_pos < head_end);
    }

    #[test]
    fn test_inject_csrf_token_no_head_tag() {
        let html = "<html><body>No head tag</body></html>";
        let token = "test-token";
        let result = inject_csrf_token(html, token);

        assert_eq!(result, html);
    }

    #[test]
    fn test_inject_csrf_token_escapes_special_chars() {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Test</title>
</head>
<body></body>
</html>"#;

        let token = "token<>\"'&";
        let result = inject_csrf_token(html, token);

        assert!(result.contains(&format!(r#"content="{}""#, token)));
    }

    #[test]
    fn test_generate_csrf_helper_script_contains_functions() {
        let script = generate_csrf_helper_script();

        assert!(script.contains("window.getCsrfToken"));
        assert!(script.contains("window.fetchWithCsrf"));
        assert!(script.contains("window.injectCsrfIntoForms"));
        assert!(script.contains("X-CSRF-Token"));
        assert!(script.contains("__csrf_token"));
    }

    #[test]
    fn test_generate_csrf_helper_script_is_valid_html() {
        let script = generate_csrf_helper_script();

        assert!(script.starts_with("<script>"));
        assert!(script.ends_with("</script>"));
    }
}
