# Test Real XAdES Signing

## Endpoint: GET /api/test/sign

This calls Rust core real signer with self-signed cert.

## How to run test

1. Start Rust core:
```bash
cd services/core
cargo run
```

2. In another terminal, call Next.js BFF which proxies to Rust:
```bash
curl http://localhost:3000/api/test/sign | jq
```

Or directly Rust:
```bash
curl -X POST http://localhost:3001/v1/test/sign-demo -H "Content-Type: application/json" -d '{}' | jq
```

## What you get

- `e_ncf`: E320000000001
- `track_id`: DEMO-TRACK-...
- `codigo_seguridad`: 6 chars from signature hash (used in QR)
- `digest_value`: base64 SHA256(c14n(original XML))
- `signature_value_preview`: first 100 chars of RSA-SHA256(signedInfo)
- `qr_url`: DGII consulta timbre URL with params
- `signed_xml_preview`: first 500 chars of signed XML with <Signature> embedded
- `signed_xml_full_base64`: full signed XML base64 for storage/S3

## Verify signature manually

Decode base64 full XML, you should see:

```xml
<ECF>...<Signature xmlns="http://www.w3.org/2000/09/xmldsig#">
  <SignedInfo>
    <CanonicalizationMethod Algorithm="http://www.w3.org/TR/2001/REC-xml-c14n-20010315"/>
    <SignatureMethod Algorithm="http://www.w3.org/2001/04/xmldsig-more#rsa-sha256"/>
    <Reference URI="">
      <Transforms><Transform Algorithm="enveloped-signature"/></Transforms>
      <DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"/>
      <DigestValue>...</DigestValue>
    </Reference>
  </SignedInfo>
  <SignatureValue>BASE64_RSA_SHA256_SIGNATURE</SignatureValue>
  <KeyInfo><X509Data><X509Certificate>BASE64_DER_CERT</X509Certificate></X509Data></KeyInfo>
</Signature></ECF>
```

This is exactly DGII spec structure from their PDF.

## Next: Use real P12

POST /api/sales with p12Base64:

```js
const p12File = await fs.readFile('cert.p12');
const p12Base64 = p12File.toString('base64');

await fetch('/api/sales', {
  method: 'POST',
  body: JSON.stringify({
    tenantId: '130793752',
    tipoECF: 32,
    items: [{sku: 'ARZ', name: 'Arroz', qty: 1, price: 1000}],
    p12Base64,
    p12Password: 'your-password'
  })
})
```
