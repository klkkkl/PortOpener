# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in PortOpener, please report it responsibly.

### How to Report

**Please DO NOT report security vulnerabilities through public GitHub issues.**

Instead, please send an email to: [your-security-email@example.com]

Include the following information:

- Type of vulnerability
- Full paths of source file(s) related to the vulnerability
- Location of the affected source code (tag/branch/commit or direct URL)
- Step-by-step instructions to reproduce the issue
- Proof-of-concept or exploit code (if possible)
- Impact of the vulnerability

### What to Expect

- **Acknowledgment**: We will acknowledge receipt of your report within 48 hours
- **Investigation**: We will investigate and validate the vulnerability
- **Fix**: We will work on a fix and coordinate disclosure timing with you
- **Credit**: We will credit you in the security advisory (unless you prefer to remain anonymous)

## Security Best Practices

When using PortOpener:

1. **Don't expose sensitive services**: Avoid forwarding database ports, admin panels, or other sensitive services to public networks
2. **Use firewall rules**: Combine PortOpener with system firewall to restrict access
3. **Monitor logs**: Regularly check connection logs for suspicious activity
4. **Keep updated**: Always use the latest version with security patches
5. **Limit listen addresses**: Use `127.0.0.1` instead of `0.0.0.0` when possible

## Known Security Considerations

### Port Forwarding Risks

- **Open ports**: Forwarding ports can expose services to network attacks
- **No authentication**: PortOpener itself doesn't provide authentication
- **No encryption**: Traffic is forwarded as-is without encryption

### Mitigation

- Use VPN or SSH tunnels for sensitive traffic
- Implement authentication at the application level
- Use TLS/SSL for encrypted connections
- Apply principle of least privilege

## Security Features

- ✅ Memory-safe implementation (Rust)
- ✅ No arbitrary code execution
- ✅ Input validation for addresses
- ✅ Graceful error handling
- ✅ Session timeout for UDP (prevents resource exhaustion)

## Disclosure Policy

- We follow responsible disclosure practices
- Security fixes are released as soon as possible
- Security advisories are published on GitHub Security Advisories
- CVE IDs are requested for significant vulnerabilities

---

**Last Updated**: 2026-03-07
