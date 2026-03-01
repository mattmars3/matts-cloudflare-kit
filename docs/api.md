# API Documentation
The toolkit primarily utilizes [HTMX](https://htmx.org/) for its API endpoints. I chose this because it greatly simplifies the amount of code needed to include these widgets on your site. This makes it far easier for beginners to incorporate the dynamic widgets into their website.

Additionally, it handles the boilerplate of substituting values into components without including the often substantial bloat of a reactive frontend framework.

Note that this guide will use my personal website API as an example. If you would like to test your personally hosted instance of the API, change the base URLs accordingly.

## Endpoints
Each endpoint will include a description and [cURL](https://curl.se/docs/manpage.html) commands to test it. **Note** that you can also test the endpoints on my personal website [mattmarsico.com](https://mattmarsico.com/).

## GET - /api/top-tracks
Returns an HTML component containing your top three Spotify songs of the month. They are ordered from most to least popular, right to left. Each track name and artist links back to the respective track and artist within Spotify. 

```bash
curl https://mattmarsico.com/api/top-tracks
```

```html
<div class="top-songs">
  <h1>Top Songs of the Month</h1>
  <div class="top-songs-item-list">
    <div class="top-songs-item">
      <img src="https://i.scdn.co/image/ab67616d0000b2734f124aa3dbcf51cf291062a0"/>
      <a href="https://open.spotify.com/track/54yMDzKjidt7XPOuCeXBV6">Watching, Waiting</a>
      <a href="https://open.spotify.com/artist/6w7j5wQ5AI5OQYlcM15s2L">Extreme</a>
    </div>
    <div class="top-songs-item">
      <img src="https://i.scdn.co/image/ab67616d0000b2730a719f2817838a22e6a7e8c9"/>
      <a href="https://open.spotify.com/track/3QVtDnXU5zqGWxWDFBMiDj">Gold Soundz</a>
      <a href="https://open.spotify.com/artist/3inCNiUr4R6XQ3W43s9Aqi">Pavement</a>
    </div>
    <div class="top-songs-item">
      <img src="https://i.scdn.co/image/ab67616d0000b2730a719f2817838a22e6a7e8c9"/>
      <a href="https://open.spotify.com/track/1CwQqhPYxz73UYKD7ybYGd">Silence Kid</a>
      <a href="https://open.spotify.com/artist/3inCNiUr4R6XQ3W43s9Aqi">Pavement</a>
    </div>
  </div>
</div>
```

## GET - /api/guestbook
Returns an multiple div tags containing signatures of the guestbook. 
```bash
curl https://mattmarsico.com/api/guestbook
```

```html
<div class="guestbook-entry">
    <p class="guestbook-entry-name">Matt Marsico:</p>
    <p class="guestbook-entry-message">Welcome to the website!</p>
    <p class="guestbook-entry-timestamp">2026-03-01 01:04:55</p>
</div>
```

## POST - /api/guestbook/sign
This endpoint is used with the form component on your page to "sign" the guestbook, or submit your name and a message.

```bash
curl -X POST http://mattmarsico.com/api/guestbook/sign \
        -H "HX-Request: true" \
        -d "name=John Smith" \
        -d "message=2 and 2 is 4"
```

```html
<div class="guestbook-entry">
    <p class="guestbook-entry-name">John Smith:</p>
    <p class="guestbook-entry-message">2 and 2 is 4</p>
    <p class="guestbook-entry-timestamp">Just Now</p>
</div>
```

## GET - /api/quote
Get a random quote from your Cloudflare D1 SQL database and display it to those who visit your website.

```bash
curl https://mattmarsico.com/api/quote
```

```html
<div class="quote-wrapper">
  <h1>Some Quotes I Like</h1>
  <div class="quote-container">
    <p class="quote-body">
      "Cause I don't care where I belong no more. What we share or not I will ignore. And I won't waste my time fittin' in. Cause I don't think contrast is a sin. No it's not a sin"
    </p>
    <p class="quote-attribution">
      No Cigar - Millencolin
    </p>
  </div>
</div>
```
