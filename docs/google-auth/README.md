# Google Auth Integration with DeepSeek

Integrate Google OAuth 2.0 authentication into your DeepSeek-powered applications to securely manage user identity and access.

## Overview

Google Auth (Google Identity Services) provides OAuth 2.0 and OpenID Connect-based authentication, allowing users to sign in to your DeepSeek application using their Google accounts. This eliminates the need to manage passwords while providing enterprise-grade security.

## Features

- **OAuth 2.0 / OpenID Connect**: Industry-standard authentication protocol
- **One Tap Sign-In**: Streamlined UX with Google One Tap
- **Access Token Management**: Secure token issuance and refresh
- **User Profile Access**: Retrieve verified email, name, and profile picture
- **Multi-platform Support**: Works on web, iOS, and Android

## Setup

### 1. Create Google OAuth Credentials

1. Go to [Google Cloud Console](https://console.cloud.google.com/)
2. Create a new project or select an existing one
3. Navigate to **APIs & Services > Credentials**
4. Click **Create Credentials > OAuth 2.0 Client ID**
5. Set the authorized redirect URIs for your application

### 2. Install Dependencies

```bash
# Node.js / Express
npm install google-auth-library

# Python
pip install google-auth google-auth-oauthlib
```

### 3. Implement Authentication

**Node.js Example:**

```javascript
const { OAuth2Client } = require('google-auth-library');
const client = new OAuth2Client(process.env.GOOGLE_CLIENT_ID);

async function verifyGoogleToken(token) {
  const ticket = await client.verifyIdToken({
    idToken: token,
    audience: process.env.GOOGLE_CLIENT_ID,
  });
  return ticket.getPayload(); // { sub, email, name, picture, ... }
}
```

**Python Example:**

```python
from google.oauth2 import id_token
from google.auth.transport import requests

def verify_google_token(token: str) -> dict:
    request = requests.Request()
    payload = id_token.verify_oauth2_token(
        token, request, GOOGLE_CLIENT_ID
    )
    return payload  # { sub, email, name, picture, ... }
```

### 4. Combine with DeepSeek API

Once the user is authenticated, use their verified identity to gate access to the DeepSeek API:

```python
import anthropic  # or openai-compatible client for DeepSeek
from deepseek import DeepSeekClient

def chat_with_deepseek(google_token: str, user_message: str) -> str:
    # Verify identity first
    user = verify_google_token(google_token)

    # Initialize DeepSeek client
    client = DeepSeekClient(api_key=DEEPSEEK_API_KEY)

    response = client.chat.completions.create(
        model="deepseek-chat",
        messages=[
            {"role": "system", "content": f"You are assisting {user['name']}."},
            {"role": "user", "content": user_message},
        ],
    )
    return response.choices[0].message.content
```

## OAuth 2.0 Flow

```
User clicks "Sign in with Google"
        │
        ▼
Google OAuth 2.0 Authorization Endpoint
(accounts.google.com/o/oauth2/v2/auth)
        │
        ▼
User grants consent
        │
        ▼
Redirect to your app with authorization code
        │
        ▼
Exchange code for ID token + access token
        │
        ▼
Verify ID token on your backend
        │
        ▼
User authenticated → Allow DeepSeek API access
```

## Environment Variables

```env
GOOGLE_CLIENT_ID=your-client-id.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=your-client-secret
DEEPSEEK_API_KEY=your-deepseek-api-key
```

## Security Best Practices

- Always verify the ID token on your **backend** — never trust client-side claims
- Validate the `aud` (audience) claim matches your `GOOGLE_CLIENT_ID`
- Use HTTPS for all redirect URIs
- Store tokens securely (httpOnly cookies or secure server-side sessions)
- Implement token expiry and refresh logic

## Resources

- [Google Identity Platform Documentation](https://developers.google.com/identity)
- [OAuth 2.0 for Web Server Applications](https://developers.google.com/identity/protocols/oauth2/web-server)
- [DeepSeek API Documentation](https://platform.deepseek.com/api-docs/)
- [google-auth-library (Node.js)](https://github.com/googleapis/google-auth-library-nodejs)
- [google-auth (Python)](https://google-auth.readthedocs.io/)
