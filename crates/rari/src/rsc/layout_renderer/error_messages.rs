use crate::server::routing::app_router::AppRouteMatch;

pub fn create_wrapped_html_error_message(
    route_match: &AppRouteMatch,
    root_layout_path: Option<&str>,
) -> String {
    let mut msg = String::from("🚨 React Hydration Mismatch Detected: Wrapped HTML Element\n\n");

    msg.push_str("╔═══════════════════════════════════════════════════════════════════════════╗\n");
    msg.push_str("║ WHAT HAPPENED:                                                            ║\n");
    msg.push_str("╚═══════════════════════════════════════════════════════════════════════════╝\n");
    msg.push_str("The root layout's <html> element is being wrapped in a container element.\n");
    msg.push_str(
        "This causes React hydration to fail because the server-rendered HTML structure\n",
    );
    msg.push_str("doesn't match what React expects on the client side.\n\n");

    msg.push_str("React hydration is the process where React attaches event listeners and makes\n");
    msg.push_str("the server-rendered HTML interactive. When the structures don't match, React\n");
    msg.push_str("can't properly hydrate, leading to frozen inputs, broken interactions, and\n");
    msg.push_str("console warnings.\n\n");

    msg.push_str("📍 AFFECTED FILES:\n");
    msg.push_str(&format!("   Route:        {}\n", route_match.route.path));
    msg.push_str(&format!("   Page:         {}\n", route_match.route.file_path));

    if let Some(layout_path) = root_layout_path {
        msg.push_str(&format!("   Root Layout:  {} ← CHECK THIS FILE\n", layout_path));
    }

    msg.push_str("\n🔧 HOW TO FIX (Step-by-Step):\n");
    msg.push_str("   Step 1: Open your root layout file");
    if let Some(layout_path) = root_layout_path {
        msg.push_str(&format!(" ({})", layout_path));
    }
    msg.push('\n');
    msg.push_str("   Step 2: Find the component's return statement\n");
    msg.push_str("   Step 3: Ensure <html> is the FIRST element returned (no wrapper divs)\n");
    msg.push_str("   Step 4: Verify <body> is a direct child of <html>\n");
    msg.push_str("   Step 5: Save the file and restart your dev server\n\n");

    msg.push_str("✅ CORRECT STRUCTURE:\n");
    msg.push_str("   export default function RootLayout({ children }) {\n");
    msg.push_str("     return (\n");
    msg.push_str("       <html lang=\"en\">  {/* ← html is the outermost element */}\n");
    msg.push_str("         <head>\n");
    msg.push_str("           <meta charSet=\"utf-8\" />\n");
    msg.push_str("         </head>\n");
    msg.push_str("         <body>{children}</body>\n");
    msg.push_str("       </html>\n");
    msg.push_str("     );\n");
    msg.push_str("   }\n\n");

    msg.push_str("❌ INCORRECT STRUCTURE (causes this error):\n");
    msg.push_str("   export default function RootLayout({ children }) {\n");
    msg.push_str("     return (\n");
    msg.push_str("       <div>  {/* ← PROBLEM: Don't wrap html in div! */}\n");
    msg.push_str("         <html lang=\"en\">\n");
    msg.push_str("           <body>{children}</body>\n");
    msg.push_str("         </html>\n");
    msg.push_str("       </div>\n");
    msg.push_str("     );\n");
    msg.push_str("   }\n\n");

    if !route_match.layouts.is_empty() {
        msg.push_str("📂 COMPONENT HIERARCHY:\n");
        msg.push_str("   Your current component tree for this route:\n\n");
        for (idx, layout) in route_match.layouts.iter().enumerate() {
            let indent = "   ".repeat(idx + 1);
            let marker = if layout.is_root { " ← ROOT LAYOUT (check this!)" } else { "" };
            msg.push_str(&format!("   {}└─ {}{}\n", indent, layout.file_path, marker));
        }
        msg.push_str(&format!(
            "   {}└─ {} (page component)\n",
            "   ".repeat(route_match.layouts.len() + 1),
            route_match.route.file_path
        ));
        msg.push('\n');
    }

    msg.push_str("💡 WHY THIS MATTERS:\n");
    msg.push_str("   • Input fields will freeze and become unresponsive\n");
    msg.push_str("   • Form submissions may fail\n");
    msg.push_str("   • Interactive elements won't work properly\n");
    msg.push_str("   • React will log hydration warnings in the browser console\n\n");

    msg.push_str("📚 LEARN MORE:\n");
    msg.push_str(
        "   • React Hydration: https://react.dev/reference/react-dom/client/hydrateRoot\n",
    );
    msg.push_str("   • Root Layout Pattern: Check your framework's documentation for root layout structure\n");

    msg
}

pub fn create_empty_rsc_error_message() -> String {
    let mut msg = String::from("🚨 RSC Streaming Error: Empty Content\n\n");

    msg.push_str("╔═══════════════════════════════════════════════════════════════════════════╗\n");
    msg.push_str("║ RENDERING ERROR:                                                          ║\n");
    msg.push_str("╚═══════════════════════════════════════════════════════════════════════════╝\n");
    msg.push_str(
        "The RSC renderer produced empty output, which cannot be streamed to the client.\n",
    );
    msg.push_str(
        "This means your component didn't render anything, or the rendering process failed.\n\n",
    );

    msg.push_str("💡 COMMON CAUSES:\n");
    msg.push_str("   1. Component returned null, undefined, or false\n");
    msg.push_str("   2. Layout or page component failed to render (check for errors)\n");
    msg.push_str("   3. RSC serialization failed silently\n");
    msg.push_str("   4. Missing 'export default' in your component file\n");
    msg.push_str("   5. Async component that threw an error\n\n");

    msg.push_str("🔧 HOW TO FIX (Step-by-Step):\n");
    msg.push_str("   Step 1: Check your server logs for any error messages\n");
    msg.push_str("   Step 2: Verify your page component exports a default function:\n");
    msg.push_str("           • File should have: export default function Page() { ... }\n");
    msg.push_str("   Step 3: Ensure your component returns valid JSX:\n");
    msg.push_str("           • Must return a React element, not null/undefined\n");
    msg.push_str("   Step 4: If using async components, verify they properly await data:\n");
    msg.push_str("           • export default async function Page() { await data; return <div>...</div>; }\n");
    msg.push_str(
        "   Step 5: Check that all layout components in the chain return valid elements\n\n",
    );

    msg.push_str("✅ VALID COMPONENT EXAMPLES:\n\n");
    msg.push_str("   Basic Page Component:\n");
    msg.push_str("   ─────────────────────\n");
    msg.push_str("   export default function Page() {\n");
    msg.push_str("     return <div>Hello World</div>;\n");
    msg.push_str("   }\n\n");

    msg.push_str("   Async Page Component:\n");
    msg.push_str("   ─────────────────────\n");
    msg.push_str("   export default async function Page() {\n");
    msg.push_str("     const data = await fetchData();\n");
    msg.push_str("     return <div>{data.title}</div>;\n");
    msg.push_str("   }\n\n");

    msg.push_str("   Layout Component:\n");
    msg.push_str("   ─────────────────\n");
    msg.push_str("   export default function Layout({ children }) {\n");
    msg.push_str("     return (\n");
    msg.push_str("       <div>\n");
    msg.push_str("         <nav>Navigation</nav>\n");
    msg.push_str("         {children}\n");
    msg.push_str("       </div>\n");
    msg.push_str("     );\n");
    msg.push_str("   }\n\n");

    msg.push_str("❌ INVALID EXAMPLES (cause this error):\n\n");
    msg.push_str("   Missing export:\n");
    msg.push_str("   ───────────────\n");
    msg.push_str("   function Page() {  // ← Missing 'export default'\n");
    msg.push_str("     return <div>Hello</div>;\n");
    msg.push_str("   }\n\n");

    msg.push_str("   Returning null:\n");
    msg.push_str("   ───────────────\n");
    msg.push_str("   export default function Page() {\n");
    msg.push_str("     return null;  // ← Don't return null\n");
    msg.push_str("   }\n\n");

    msg.push_str("🔍 DEBUGGING CHECKLIST:\n");
    msg.push_str("   □ Component file has 'export default'\n");
    msg.push_str("   □ Component returns JSX (not null/undefined)\n");
    msg.push_str("   □ No errors in server logs\n");
    msg.push_str("   □ Async components properly await data\n");
    msg.push_str("   □ All layout components in the chain are valid\n");

    msg
}

pub fn create_invalid_rsc_format_warning(issue: &str, details: &str) -> String {
    let mut msg = String::from("⚠️  RSC Wire Format Warning: Invalid Structure\n\n");

    msg.push_str("╔═══════════════════════════════════════════════════════════════════════════╗\n");
    msg.push_str("║ SERIALIZATION WARNING:                                                    ║\n");
    msg.push_str("╚═══════════════════════════════════════════════════════════════════════════╝\n");
    msg.push_str(&format!("Issue:   {}\n", issue));
    msg.push_str(&format!("Details: {}\n\n", details));

    msg.push_str("The React Server Component (RSC) wire format appears to be malformed.\n");
    msg.push_str(
        "This could prevent the client from properly parsing and hydrating your components.\n\n",
    );

    msg.push_str("📋 RSC WIRE FORMAT SPECIFICATION:\n");
    msg.push_str("   The React Server Component wire format should follow this structure:\n\n");
    msg.push_str("   • Each line starts with a numeric row ID followed by a colon\n");
    msg.push_str("     Example: 0:[...]\n\n");
    msg.push_str("   • React elements are represented as JSON arrays:\n");
    msg.push_str("     Format: [\"$\", \"tagName\", key, props]\n");
    msg.push_str("     Example: [\"$\", \"div\", null, {\"className\": \"container\"}]\n\n");
    msg.push_str("   • Multiple rows are separated by newlines\n");
    msg.push_str("   • Props can contain nested elements (children)\n\n");

    msg.push_str("✅ VALID RSC WIRE FORMAT EXAMPLES:\n\n");
    msg.push_str("   Simple element:\n");
    msg.push_str("   0:[\"$\",\"div\",null,{\"children\":\"Hello World\"}]\n\n");

    msg.push_str("   Nested structure:\n");
    msg.push_str("   0:[\"$\",\"html\",null,{\"lang\":\"en\",\"children\":[\"$\",\"body\",null,{\"children\":\"...\"}]}]\n\n");

    msg.push_str("   Multiple rows:\n");
    msg.push_str("   0:[\"$\",\"div\",null,{\"children\":[\"$L1\"]}]\n");
    msg.push_str("   1:[\"$\",\"span\",null,{\"children\":\"Text\"}]\n\n");

    msg.push_str("💡 WHAT THIS WARNING MAY INDICATE:\n");
    msg.push_str("   • A bug in the RSC serializer (internal framework issue)\n");
    msg.push_str("   • Corrupted component output from a malformed component\n");
    msg.push_str("   • Incompatible React version or missing dependencies\n");
    msg.push_str("   • Component returning invalid data types\n\n");

    msg.push_str("🔧 TROUBLESHOOTING STEPS:\n");
    msg.push_str("   Step 1: Check the server logs for any serialization errors\n");
    msg.push_str("   Step 2: Verify your components return valid React elements:\n");
    msg.push_str("           • Should return JSX, not plain objects or strings\n");
    msg.push_str("   Step 3: Ensure you're using a compatible version of React:\n");
    msg.push_str("           • Check package.json for react and react-dom versions\n");
    msg.push_str("   Step 4: Look for components that might return unexpected values:\n");
    msg.push_str("           • Avoid returning raw objects, arrays, or primitives at the root\n");
    msg.push_str("   Step 5: If the issue persists, this may be a framework bug:\n");
    msg.push_str("           • Report the issue with the full error details\n\n");

    msg.push_str("🔍 DEBUGGING TIPS:\n");
    msg.push_str("   • Enable verbose logging to see the full RSC output\n");
    msg.push_str("   • Check if specific components cause the issue (comment them out)\n");
    msg.push_str("   • Verify all imports are correct and components are properly defined\n");
    msg.push_str("   • Test with a minimal component to isolate the problem\n");

    msg
}
