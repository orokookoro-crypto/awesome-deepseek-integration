# Google OAuth 2.0 Web Server + DeepSeek Integration

This guide shows how to build a web application that uses the **Google OAuth 2.0 Web Server flow** to access Google Workspace data (Drive, Gmail, Calendar, etc.) and processes it with the **DeepSeek API**.

## How It Works

The OAuth 2.0 Web Server flow is designed for server-side applications that can securely store a Client Secret. It follows five steps:

1. **Obtain Credentials** — Get a `Client ID` and `Client Secret` from the [Google Cloud Console](https://console.cloud.google.com/).
2. **Redirect to Google** — Send the user to Google's authorization endpoint with the required scopes.
3. **User Consent** — The user approves your requested permissions on Google's consent screen.
4. **Exchange Code for Tokens** — Google returns a short-lived Authorization Code; your server exchanges it (with the Client Secret) for an **Access Token** and a **Refresh Token**.
5. **Call Google APIs** — Use the Access Token to fetch data, then pass it to DeepSeek for AI-powered analysis.

| Token | Purpose |
|---|---|
| **Access Token** | Short-lived credential (~1 hour) for calling Google APIs |
| **Refresh Token** | Long-lived credential for obtaining new Access Tokens |
| **Scopes** | Specific permissions, e.g. `https://www.googleapis.com/auth/drive.readonly` |
| **Redirect URI** | The exact URL in your app where Google sends the authorization code |

## Prerequisites

- A [Google Cloud project](https://console.cloud.google.com/) with OAuth 2.0 credentials configured
- A [DeepSeek API key](https://platform.deepseek.com/)
- Python 3.9+ with `pip`

## Setup

### 1. Install dependencies

```bash
pip install flask google-auth google-auth-oauthlib google-api-python-client openai
```

### 2. Configure Google Cloud credentials

In the [Google Cloud Console](https://console.cloud.google.com/):
1. Go to **APIs & Services → Credentials**
2. Create an **OAuth 2.0 Client ID** of type *Web application*
3. Add `http://localhost:5000/oauth2callback` to **Authorized redirect URIs**
4. Download the `client_secret.json` file

### 3. Set environment variables

```bash
export DEEPSEEK_API_KEY="your_deepseek_api_key"
```

## Example: AI-Powered Google Drive Summarizer

This Flask application authenticates the user via Google OAuth, lists their recent Drive files, and uses DeepSeek to summarize them.

```python
import os
import json
from flask import Flask, redirect, request, session, url_for
from google_auth_oauthlib.flow import Flow
from google.oauth2.credentials import Credentials
from googleapiclient.discovery import build
from openai import OpenAI

app = Flask(__name__)
app.secret_key = os.urandom(24)

# Allow HTTP for local development only
os.environ["OAUTHLIB_INSECURE_TRANSPORT"] = "1"

SCOPES = ["https://www.googleapis.com/auth/drive.metadata.readonly"]
CLIENT_SECRETS_FILE = "client_secret.json"

deepseek = OpenAI(
    api_key=os.environ["DEEPSEEK_API_KEY"],
    base_url="https://api.deepseek.com",
)


@app.route("/")
def index():
    if "credentials" not in session:
        return '<a href="/authorize">Connect Google Drive</a>'
    return redirect(url_for("summarize"))


@app.route("/authorize")
def authorize():
    flow = Flow.from_client_secrets_file(
        CLIENT_SECRETS_FILE,
        scopes=SCOPES,
        redirect_uri=url_for("oauth2callback", _external=True),
    )
    # Request offline access to receive a Refresh Token
    authorization_url, state = flow.authorization_url(
        access_type="offline",
        include_granted_scopes="true",
    )
    session["state"] = state
    return redirect(authorization_url)


@app.route("/oauth2callback")
def oauth2callback():
    flow = Flow.from_client_secrets_file(
        CLIENT_SECRETS_FILE,
        scopes=SCOPES,
        state=session["state"],
        redirect_uri=url_for("oauth2callback", _external=True),
    )
    flow.fetch_token(authorization_response=request.url)

    credentials = flow.credentials
    session["credentials"] = {
        "token": credentials.token,
        "refresh_token": credentials.refresh_token,
        "token_uri": credentials.token_uri,
        "client_id": credentials.client_id,
        "client_secret": credentials.client_secret,
        "scopes": credentials.scopes,
    }
    return redirect(url_for("summarize"))


@app.route("/summarize")
def summarize():
    if "credentials" not in session:
        return redirect(url_for("authorize"))

    creds = Credentials(**session["credentials"])
    drive = build("drive", "v3", credentials=creds)

    # Fetch the 10 most recently modified files
    results = drive.files().list(
        pageSize=10,
        orderBy="modifiedTime desc",
        fields="files(id, name, mimeType, modifiedTime)",
    ).execute()
    files = results.get("files", [])

    if not files:
        return "No files found in Google Drive."

    file_list = "\n".join(
        f"- {f['name']} ({f['mimeType']}, modified {f['modifiedTime']})"
        for f in files
    )

    # Use DeepSeek to summarize the file list
    response = deepseek.chat.completions.create(
        model="deepseek-chat",
        messages=[
            {
                "role": "system",
                "content": "You are a helpful assistant that summarizes Google Drive activity.",
            },
            {
                "role": "user",
                "content": (
                    "Here are the 10 most recently modified files in this user's "
                    f"Google Drive:\n\n{file_list}\n\n"
                    "Please provide a brief summary of recent activity and any patterns you notice."
                ),
            },
        ],
    )

    summary = response.choices[0].message.content
    return f"<h2>Recent Drive Activity Summary</h2><pre>{summary}</pre>"


@app.route("/revoke")
def revoke():
    session.clear()
    return "Credentials cleared. <a href='/'>Start over</a>"


if __name__ == "__main__":
    app.run(debug=True)
```

## Running the Example

```bash
python app.py
```

Open `http://localhost:5000` in your browser. Click **Connect Google Drive**, approve the permissions, and DeepSeek will summarize your recent activity.

## Security Best Practices

- **Never commit** `client_secret.json` or your DeepSeek API key to version control.
- Store the Refresh Token securely (encrypted database, secrets manager) — it provides long-term access.
- Request only the **minimum scopes** needed (Incremental Authorization).
- In production, use HTTPS and remove `OAUTHLIB_INSECURE_TRANSPORT`.
- Rotate your Client Secret periodically and revoke tokens when a user disconnects.

## Extending This Integration

| Google API | Example Use Case with DeepSeek |
|---|---|
| Gmail | Triage and summarize unread emails |
| Calendar | Analyze scheduling patterns and suggest optimizations |
| Drive | Classify and tag documents automatically |
| Docs | Review and improve document content |
| Sheets | Interpret data and generate natural-language reports |
