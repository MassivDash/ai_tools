const http = require('http');
const url = require('url');

const CLIENT_ID = process.env.GOOGLE_CLIENT_ID;
const CLIENT_SECRET = process.env.GOOGLE_CLIENT_SECRET;
const REDIRECT_URI = 'http://localhost:3000';

if (!CLIENT_ID || !CLIENT_SECRET) {
    console.error('❌ Error: GOOGLE_CLIENT_ID and GOOGLE_CLIENT_SECRET must be set as environment variables.');
    process.exit(1);
}

const SCOPES = [
    'https://www.googleapis.com/auth/gmail.send',
    'https://www.googleapis.com/auth/gmail.readonly',
    'https://www.googleapis.com/auth/calendar.events'
].join(' ');

const AUTH_URL = `https://accounts.google.com/o/oauth2/v2/auth?client_id=${CLIENT_ID}&redirect_uri=${encodeURIComponent(REDIRECT_URI)}&response_type=code&scope=${encodeURIComponent(SCOPES)}&access_type=offline&prompt=consent`;

console.log('\n=============================================');
console.log('🔗 Please open this URL in your browser:');
console.log(AUTH_URL);
console.log('=============================================\n');

const server = http.createServer(async (req, res) => {
    try {
        const reqUrl = url.parse(req.url, true);
        
        if (reqUrl.pathname === '/') {
            const code = reqUrl.query.code;
            
            if (code) {
                res.writeHead(200, { 'Content-Type': 'text/html' });
                res.end('<h1>Authentication successful!</h1><p>You can close this window and return to your terminal.</p>');
                
                console.log('🔄 Received authorization code. Exchanging for tokens...');
                
                const tokenResponse = await fetch('https://oauth2.googleapis.com/token', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
                    body: new URLSearchParams({
                        code: code,
                        client_id: CLIENT_ID,
                        client_secret: CLIENT_SECRET,
                        redirect_uri: REDIRECT_URI,
                        grant_type: 'authorization_code'
                    })
                });
                
                const tokenData = await tokenResponse.json();
                
                if (tokenData.refresh_token) {
                    console.log('\n✅ Successfully obtained Refresh Token!\n');
                    console.log('Add this exact line to your .env file:');
                    console.log(`GOOGLE_REFRESH_TOKEN=${tokenData.refresh_token}`);
                    console.log('\n(You can now safely exit this script with Ctrl+C)');
                } else {
                    console.error('❌ Error: Response did not contain a refresh token.', tokenData);
                    console.log('Did you forget prompt=consent? You might need to remove the app from your Google account permissions and try again.');
                }
                
                server.close();
                process.exit(0);
            } else {
                res.writeHead(400, { 'Content-Type': 'text/plain' });
                res.end('Error: No code provided');
            }
        }
    } catch (e) {
        console.error(e);
        res.writeHead(500);
        res.end('Internal Server Error');
    }
});

server.listen(3000, () => {
    console.log('👂 Listening on http://localhost:3000 for the callback...');
});
