    keytool -genkeypair -alias indyzai -keyalg RSA -keysize 2048 -validity 10000 -keystore indyzai.keystore


java -jar pepk.jar --keystore=indyzai.keystore --alias=indyzai --output=indyzai.zip --include-cert --rsa-aes-encryption --encryption-key-path=./encryption_public_key.pem

base64 < indyzai.keystore > indyzai.keystore.base64 


ANDROID_KEYSTORE_BASE64: indyzai.keystore.base64
ANDROID_STORE_PASSWORD: [PASSWORD]
ANDROID_KEY_ALIAS: indyzai
ANDROID_KEY_PASSWORD: [PASSWORD]