/**
 * Móvil POS - App Expo
 * Conecta al mismo núcleo Rust vía gRPC
 * Comparte @repo/api-client
 * UI 100% Español Dominicano
 */
import { Text, View, ScrollView, TouchableOpacity } from "react-native";

export default function POSMovil() {
  return (
    <ScrollView style={{ flex: 1, backgroundColor: "#09090b" }} contentContainerStyle={{ padding: 20, paddingTop: 60 }}>
      <Text style={{ color: "white", fontSize: 24, fontWeight: "800" }}>Colmado POS Móvil</Text>
      <Text style={{ color: "#a1a1aa", marginTop: 6, fontSize: 12 }}>Núcleo bancario Rust • TigerBeetle • DGII e-CF • Adelantos 50%</Text>
      
      <View style={{ marginTop: 20, backgroundColor: "#18181b", borderRadius: 16, padding: 16, borderWidth: 1, borderColor: "#27272a" }}>
        <Text style={{ color: "#f4f4f5", fontSize: 12, fontWeight: "600", letterSpacing: 1 }}>EMPLEADO</Text>
        <Text style={{ color: "white", fontSize: 18, fontWeight: "700", marginTop: 4 }}>Juan Carlos • Almacén</Text>
        <View style={{ marginTop: 12, backgroundColor: "#09090b", borderRadius: 12, padding: 12, borderWidth: 1, borderColor: "#27272a" }}>
          <Text style={{ color: "#a1a1aa", fontSize: 11 }}>Sueldo ganado hoy (acumulado neto)</Text>
          <Text style={{ color: "#22c55e", fontSize: 20, fontWeight: "800", marginTop: 2 }}>RD$ 6,400.00</Text>
          <Text style={{ color: "#71717a", fontSize: 10, marginTop: 6 }}>Horas: 8h • Tarifa: RD$800 • TSS -5.91% estimado</Text>
        </View>

        <View style={{ marginTop: 12, backgroundColor: "#422006", borderRadius: 12, padding: 12, borderWidth: 1, borderColor: "#78350f" }}>
          <Text style={{ color: "#fde68a", fontSize: 11, fontWeight: "600" }}>DISPONIBLE PARA ADELANTO • REGLA 50%</Text>
          <Text style={{ color: "#fbbf24", fontSize: 22, fontWeight: "800", marginTop: 4 }}>RD$ 3,200.00</Text>
          <Text style={{ color: "#d6d3d1", fontSize: 10, marginTop: 4 }}>Máximo permitido: 50% del neto ganado. Sin interés. Descuento automático en quincena como "Anticipo de Salario".</Text>
        </View>

        <TouchableOpacity style={{ backgroundColor: "white", marginTop: 14, padding: 14, borderRadius: 12, alignItems: "center" }}>
          <Text style={{ color: "black", fontWeight: "800", fontSize: 14 }}>Solicitar Adelanto RD$ 2,000.00</Text>
          <Text style={{ color: "#52525b", fontSize: 10, marginTop: 2 }}>Motivo: Transporte</Text>
        </TouchableOpacity>

        <Text style={{ color: "#52525b", fontSize: 9, marginTop: 10, textAlign: "center", lineHeight: 14 }}>
          Flujo bancario: Evento AdelantoSolicitado → Núcleo Rust verifica 50% → TigerBeetle reserva pending (Debe anticipos / Haber caja) → Gerente aprueba → posted → Evento AdelantoAprobado
        </Text>
      </View>

      <View style={{ marginTop: 18, backgroundColor: "#18181b", borderRadius: 16, padding: 16, borderWidth: 1, borderColor: "#27272a" }}>
        <Text style={{ color: "white", fontSize: 14, fontWeight: "700" }}>Venta Rápida • E32 Consumidor Final</Text>
        <Text style={{ color: "#71717a", fontSize: 11, marginTop: 4 }}>eNCF: E320000000129 • RFCE Activo (&lt; RD$250k) • ITBIS 18%/16%/Exento</Text>
        
        <View style={{ marginTop: 12, flexDirection: "row", gap: 8 }}>
          <View style={{ flex: 1, backgroundColor: "#09090b", borderRadius: 10, padding: 10, borderWidth: 1, borderColor: "#27272a" }}>
            <Text style={{ color: "#71717a", fontSize: 10 }}>Arroz Premium</Text>
            <Text style={{ color: "white", fontSize: 13, fontWeight: "700", marginTop: 2 }}>RD$ 118.00</Text>
            <Text style={{ color: "#52525b", fontSize: 9 }}>18% ITBIS</Text>
          </View>
          <View style={{ flex: 1, backgroundColor: "#09090b", borderRadius: 10, padding: 10, borderWidth: 1, borderColor: "#27272a" }}>
            <Text style={{ color: "#71717a", fontSize: 10 }}>Plátanos x lb</Text>
            <Text style={{ color: "white", fontSize: 13, fontWeight: "700", marginTop: 2 }}>RD$ 45.00</Text>
            <Text style={{ color: "#52525b", fontSize: 9 }}>EXENTO</Text>
          </View>
        </View>

        <TouchableOpacity style={{ backgroundColor: "#f4f4f5", marginTop: 14, padding: 14, borderRadius: 12, alignItems: "center" }}>
          <Text style={{ color: "black", fontWeight: "800" }}>Cobrar RD$ 341.34 • Generar E32 • QR DGII</Text>
        </TouchableOpacity>
      </View>

      <View style={{ marginTop: 20, borderWidth: 1, borderColor: "#27272a", borderRadius: 12, padding: 12, backgroundColor: "#18181b" }}>
        <Text style={{ color: "#a1a1aa", fontSize: 11, fontWeight: "600" }}>ESTADO DGII</Text>
        <Text style={{ color: "#22c55e", fontSize: 12, marginTop: 4 }}>● 46 facturas ACEPTADAS • 1 pendiente TrackID</Text>
        <Text style={{ color: "#71717a", fontSize: 10, marginTop: 2 }}>Último: E320000000128 • TrackID DGII-982x • Código Seg: A1B2C3 • QR: https://ecf.dgii.gov.do/eCF/...</Text>
      </View>
    </ScrollView>
  );
}
