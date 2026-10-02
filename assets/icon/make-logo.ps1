# Generates the YiJu (译句) logo: PNG master + multi-size ICO (PNG-compressed entries).
Add-Type -AssemblyName System.Drawing
$ErrorActionPreference = 'Stop'

function New-Logo([int]$size, [bool]$badge) {
    $bmp = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'; $g.TextRenderingHint = 'AntiAliasGridFit'; $g.InterpolationMode = 'HighQualityBicubic'
    $g.Clear([System.Drawing.Color]::Transparent)
    $m = [Math]::Max(1, [int]($size * 0.04)); $s = $size - 2 * $m; $r = [single]($s * 0.22)
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $d = 2 * $r
    $path.AddArc($m, $m, $d, $d, 180, 90); $path.AddArc($m + $s - $d, $m, $d, $d, 270, 90)
    $path.AddArc($m + $s - $d, $m + $s - $d, $d, $d, 0, 90); $path.AddArc($m, $m + $s - $d, $d, $d, 90, 90); $path.CloseFigure()
    $rect = New-Object System.Drawing.RectangleF($m, $m, $s, $s)
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush($rect, [System.Drawing.Color]::FromArgb(255, 33, 150, 243), [System.Drawing.Color]::FromArgb(255, 94, 53, 177), 45)
    $g.FillPath($brush, $path)
    $white = [System.Drawing.Brushes]::White
    $fmt = New-Object System.Drawing.StringFormat; $fmt.Alignment = 'Center'; $fmt.LineAlignment = 'Center'
    $font = New-Object System.Drawing.Font('Microsoft YaHei UI', [single]($s * 0.60), [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
    $shift = if ($badge) { -$s * 0.06 } else { 0 }
    $g.DrawString('译', $font, $white, (New-Object System.Drawing.RectangleF(($m + $shift), ($m + $shift + $s * 0.03), $s, $s)), $fmt)
    if ($badge) {
        # Speech-bubble badge with an "A" at the bottom right: Chinese in, English out.
        $bw = $s * 0.36; $bx = $m + $s - $bw - $s * 0.07; $by = $m + $s - $bw - $s * 0.07
        # A ring in the background gradient keeps the bubble from touching the glyph.
        $gap = $bw * 0.09
        $g.FillEllipse($brush, [single]($bx - $gap), [single]($by - $gap), [single]($bw + 2 * $gap), [single]($bw + 2 * $gap))
        $bubble = New-Object System.Drawing.Drawing2D.GraphicsPath
        $bubble.AddEllipse([single]$bx, [single]$by, [single]$bw, [single]$bw)
        $g.FillPath($white, $bubble)
        $tail = [System.Drawing.PointF[]]@(
            (New-Object System.Drawing.PointF(($bx + $bw * 0.10), ($by + $bw * 0.62))),
            (New-Object System.Drawing.PointF(($bx - $bw * 0.12), ($by + $bw * 1.00))),
            (New-Object System.Drawing.PointF(($bx + $bw * 0.40), ($by + $bw * 0.92))))
        $g.FillPolygon($white, $tail)
        $afont = New-Object System.Drawing.Font('Segoe UI', [single]($bw * 0.62), [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
        $abrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 94, 53, 177))
        $g.DrawString('A', $afont, $abrush, (New-Object System.Drawing.RectangleF([single]$bx, [single]($by + $bw * 0.03), [single]$bw, [single]$bw)), $fmt)
    }
    $g.Dispose()
    return $bmp
}

function Get-PngBytes($bmp) { $ms = New-Object System.IO.MemoryStream; $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png); $ms.ToArray() }

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$master = New-Logo 866 $true
$master.Save("$root\assets\icon\logo.png", [System.Drawing.Imaging.ImageFormat]::Png)

# ICO: header + directory + PNG payloads. Badge only from 48 px up; small sizes keep just the glyph.
$sizes = 16, 20, 24, 32, 40, 48, 64, 128, 256
$images = foreach ($size in $sizes) { , (Get-PngBytes (New-Logo $size ($size -ge 48))) }
$ms = New-Object System.IO.MemoryStream; $w = New-Object System.IO.BinaryWriter($ms)
$w.Write([UInt16]0); $w.Write([UInt16]1); $w.Write([UInt16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $dim = if ($sizes[$i] -ge 256) { 0 } else { $sizes[$i] }
    $w.Write([byte]$dim); $w.Write([byte]$dim); $w.Write([byte]0); $w.Write([byte]0)
    $w.Write([UInt16]1); $w.Write([UInt16]32); $w.Write([UInt32]$images[$i].Length); $w.Write([UInt32]$offset)
    $offset += $images[$i].Length
}
foreach ($img in $images) { $w.Write([byte[]]$img) }
$w.Flush()
[System.IO.File]::WriteAllBytes("$root\apps\windows\tsf\resources\qingjian.ico", $ms.ToArray())
"ok"
