# PowerShell script to open browser and navigate to truetype_test demo
# This will open the URL in your default browser

$url = "http://localhost:3000/#/truetype_test"

Write-Host "Opening browser to: $url"
Write-Host "Please wait 3 seconds for the page to load, then check:"
Write-Host "  1. Is there text rendered on the canvas?"
Write-Host "  2. Can you see multiple paragraphs about LCD subpixel rendering?"
Write-Host "  3. Are the controls (Font Scale, Faux Weight, etc.) visible?"
Write-Host ""

Start-Process $url

Write-Host "Browser opened. Please manually verify the demo is working."
