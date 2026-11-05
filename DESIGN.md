# Latchix admin UI design

- Status: Draft
- Date: 2026-11-05
- Scope: Optional server-rendered admin UI
- Gate: This document must be accepted before template work starts.

## Purpose

The UI is an operator tool for tenants, users, groups, roots, grants, keys,
audit, and quotas. It is not a marketing surface and not a file browser. The
API is the primary interface. The UI is a thin, opt-in client of the same
tenant-scoped API.

## Constraints

- Server-rendered HTML. No JavaScript framework in v0.1.
- No CDN assets. No external fonts. No third-party scripts.
- Works with keyboard only.
- Meets WCAG 2.2 AA contrast and focus requirements.
- No secret, token, or private key is ever rendered after creation.
- Every mutation is a normal form POST with CSRF protection and an audit event.
- The UI is compiled behind the `ui` Cargo feature and disabled by default.
- The UI never becomes the only way to perform an operation.

## Visual direction

Not yet chosen. The first UI slice must pick a direction before templates are
written. The direction must fit these constraints:

- Dense, calm, and legible. This is an operations tool, not a dashboard
  showcase.
- System font stack. No display fonts.
- One accent color plus neutral surfaces. Status colors are reserved for
  status.
- Light and dark themes. Both meet contrast requirements.
- No decorative gradients, glass panels, or illustration.
- No fake charts. Tables and plain numbers first.
- Empty states explain the next action.

## Information architecture

Top-level sections:

- Tenants
- Users
- Groups
- Roots
- Grants
- Keys
- Audit
- Settings

Tenant context is always visible. A server admin can switch tenants. A tenant
admin cannot see the switcher.

## Interaction patterns

- List pages: filter, sort, paginate, and show the total count.
- Detail pages: identity, status, quotas, grants, keys, and recent audit.
- Create and edit forms: explicit labels, inline errors, and a cancel action.
- Destructive actions: confirmation with the object name and a typed
  confirmation for tenant and root deletion.
- Grant editor: show the effective rights and the ancestor `TRAVERSE` rule.
- Revocation: show the next-operation effect, not "after re-login".
- Errors: plain language, a stable error code, and a request ID.
- Loading: server-rendered, no skeleton animation.

## Accessibility

- Every control has a visible label.
- Focus is visible and follows DOM order.
- Tables have captions and header scopes.
- Forms use fieldsets and legends for groups.
- Error summaries link to the invalid field.
- Color is never the only status signal.
- Motion respects `prefers-reduced-motion`.

## Security

- Session cookie: `HttpOnly`, `Secure` when TLS is terminated, `SameSite=Lax`.
- CSRF token per session and per form.
- No inline event handlers.
- Content Security Policy with no `unsafe-inline`.
- No tenant data in URLs beyond opaque IDs.
- No secret display after creation. One-time reveal only.
- Logout clears the session server-side.
- UI routes use the API authorization path, not a parallel check.

## Anti-slop rules

- No hero metric cards.
- No gradient buttons.
- No icon-only destructive actions.
- No "insights" or "AI summary" panel.
- No fake activity feed.
- No placeholder copy in shipped templates.
- No animated counters.
- No modal for a task that fits on a page.

## Open questions

1. Visual direction and brand palette.
2. Tenant switcher placement for server admins.
3. Whether audit uses a cursor page or a time-range filter first.
4. How to show effective permissions without a wall of checkboxes.
5. Local login versus bootstrap-token-only admin sessions.

## Approval gate

No templates, CSS, or UI routes are written until this document is accepted.
The first UI slice is users and keys, followed by grants and audit.
