# Fuentes de respaldo

La fuente principal de egui se conserva. Estas fuentes sólo cubren glifos que le faltan:

- Noto Sans Math: letras matemáticas Unicode como `𝗨𝗡 𝗛𝗘𝗖𝗛𝗜𝗖𝗘𝗥𝗢`.
- Noto Sans SC: chino y caracteres CJK; instancia regular de peso 400.
- Noto Sans Symbols 2: símbolos adicionales, incluidos los que faltan en instalaciones Linux.

Origen: [Google Fonts](https://github.com/google/fonts/tree/5e8a3ba899557829a76cfdac30fa512bda91d7ca/ofl), directorios `notosansmath`, `notosanssc` y `notosanssymbols2`. Los archivos `*-OFL.txt` contienen sus avisos de copyright y licencia SIL OFL 1.1. La instancia SC se creó con fontTools 4.65.0:

```python
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont
font = TTFont("NotoSansSC[wght].ttf")
instantiateVariableFont(font, {"wght": 400}, inplace=True).save("NotoSansSC.ttf")
```

No se puede prometer cobertura de todos los caracteres de Unicode. Las pruebas verifican las letras matemáticas solicitadas, chino y cirílico; los sistemas pueden proporcionar además sus fuentes de emojis.
