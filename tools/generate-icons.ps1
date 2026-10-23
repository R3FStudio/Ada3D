# Regenerate checked-in icons from the small polygon/polyline SVG mark.
# Run on Windows with PowerShell; building Ada does not require this script.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Split-Path -Parent $PSScriptRoot
$brand = Join-Path $root 'assets/brand'
$iconset = Join-Path $root 'assets/macos/Ada3D.iconset'
New-Item -ItemType Directory -Path $iconset -Force | Out-Null
[xml]$svg = Get-Content -LiteralPath (Join-Path $brand 'mark.svg') -Raw

function Render-Mark([int]$size) {
    $large = [System.Drawing.Bitmap]::new($size * 4, $size * 4)
    $graphics = [System.Drawing.Graphics]::FromImage($large)
    try {
        $graphics.Clear([System.Drawing.Color]::Transparent)
        $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $graphics.ScaleTransform($size / 16.0, $size / 16.0)
        foreach ($shape in $svg.DocumentElement.ChildNodes) {
            if ($shape -isnot [System.Xml.XmlElement]) { continue }
            $points = [System.Drawing.PointF[]]@(
                foreach ($pair in ($shape.GetAttribute('points') -split '\s+')) {
                    $xy = $pair -split ','
                    [System.Drawing.PointF]::new([float]$xy[0], [float]$xy[1])
                }
            )
            switch ($shape.LocalName) {
                'polygon' {
                    $brush = [System.Drawing.SolidBrush]::new([System.Drawing.ColorTranslator]::FromHtml($shape.GetAttribute('fill')))
                    try { $graphics.FillPolygon($brush, $points) } finally { $brush.Dispose() }
                }
                'polyline' {
                    $pen = [System.Drawing.Pen]::new([System.Drawing.ColorTranslator]::FromHtml($shape.GetAttribute('stroke')), [float]$shape.GetAttribute('stroke-width'))
                    $pen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
                    try { $graphics.DrawLines($pen, $points) } finally { $pen.Dispose() }
                }
                default { throw "Unsupported mark element: $($shape.LocalName)" }
            }
        }
        $result = [System.Drawing.Bitmap]::new($size, $size)
        $downsample = [System.Drawing.Graphics]::FromImage($result)
        try {
            $downsample.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
            $downsample.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $downsample.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
            $downsample.DrawImage($large, [System.Drawing.Rectangle]::new(0, 0, $size, $size))
        } finally { $downsample.Dispose() }
        $stream = [System.IO.MemoryStream]::new()
        try {
            $result.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
            return ,$stream.ToArray()
        } finally { $stream.Dispose(); $result.Dispose() }
    } finally { $graphics.Dispose(); $large.Dispose() }
}

$images = @{}
foreach ($size in @(16, 24, 32, 48, 64, 128, 256, 512, 1024)) {
    $images[$size] = Render-Mark $size
}
[System.IO.File]::WriteAllBytes((Join-Path $brand 'app-icon.png'), $images[256])
# PNG-backed ICO frames keep alpha at every Windows display scale.
$sizes = @(16, 24, 32, 48, 64, 128, 256)
$stream = [System.IO.MemoryStream]::new()
$writer = [System.IO.BinaryWriter]::new($stream)
try {
    $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$sizes.Count)
    $offset = 6 + 16 * $sizes.Count
    foreach ($size in $sizes) {
        $dimension = if ($size -eq 256) { 0 } else { $size }
        $writer.Write([byte]$dimension); $writer.Write([byte]$dimension)
        $writer.Write([byte]0); $writer.Write([byte]0)
        $writer.Write([uint16]1); $writer.Write([uint16]32)
        $writer.Write([uint32]$images[$size].Length); $writer.Write([uint32]$offset)
        $offset += $images[$size].Length
    }
    foreach ($size in $sizes) { $writer.Write([byte[]]$images[$size]) }
    [System.IO.File]::WriteAllBytes((Join-Path $brand 'app.ico'), $stream.ToArray())
} finally { $writer.Dispose(); $stream.Dispose() }
foreach ($size in @(16, 32, 128, 256, 512)) {
    [System.IO.File]::WriteAllBytes((Join-Path $iconset "icon_${size}x${size}.png"), $images[$size])
    [System.IO.File]::WriteAllBytes((Join-Path $iconset "icon_${size}x${size}@2x.png"), $images[($size * 2)])
}
Write-Output 'Generated transparent PNG, Windows ICO, and macOS iconset from mark.svg.'
