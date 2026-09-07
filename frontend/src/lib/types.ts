export interface HealthResponse {
  status: string;
  version: string;
  subsystems?: Record<string, string>;
}

export interface BuyerProfile {
  id: string;
  name: string;
  email: string;
  phone: string;
  is_phone_verified: boolean;
  created_at: string;
}

export interface LoginResponse {
  token: string;
  token_type: string;
  expires_in: number;
  buyer: BuyerProfile;
}

export interface Product {
  id: string;
  name: string;
  description: string;
  price_cents: number;
  stock: number;
  image_url?: string | null;
  category?: string | null;
  created_at?: string;
}

export interface CatalogPageResponse {
  items: Product[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export interface ProductVariant {
  id: string;
  product_id: string;
  variant_name: string;
  variant_value: string;
  sku?: string | null;
  price_override?: number | null;
  stock_quantity: number;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface CreateVariantPayload {
  variant_name: string;
  variant_value: string;
  sku?: string | null;
  price_override?: number | null;
  stock_quantity: number;
}

export interface CartItem {
  product: Product;
  quantity: number;
  variant?: ProductVariant;
}

export interface CreateOrderPayload {
  items: { product_id: string; quantity: number }[];
  notes?: string;
  shipping_address_id?: string;
}

export interface CreateOrderResponse {
  order_id: string;
  total_amount_cents: number;
  snap_token?: string;
  redirect_url?: string;
  status: string;
}

export interface BuyerOrder {
  id: string;
  total_amount_cents: number;
  status: 'PENDING_PAYMENT' | 'PAID' | 'PROCESSING' | 'SHIPPED' | 'DELIVERED' | 'CANCELLED';
  tracking_number?: string | null;
  items: {
    product_id: string;
    product_name: string;
    quantity: number;
    price_cents: number;
  }[];
  created_at: string;
}

export interface ChatMessage {
  id: string;
  sender_role: 'Buyer' | 'Seller' | 'Admin';
  sender_name: string;
  content: string;
  created_at: string;
}

export type DiscountType = 'PERCENTAGE' | 'FIXED_AMOUNT';

export interface Coupon {
  id: string;
  code: string;
  discount_type: DiscountType;
  discount_value: number;
  min_order_amount: number;
  max_discount_amount?: number | null;
  usage_limit?: number | null;
  usage_count: number;
  start_date: string;
  end_date: string;
  is_active: boolean;
  created_at: string;
}

export interface CreateCouponPayload {
  code: string;
  discount_type: DiscountType;
  discount_value: number;
  min_order_amount?: number;
  max_discount_amount?: number;
  usage_limit?: number;
  start_date: string;
  end_date: string;
}

export interface ValidateCouponPayload {
  code: string;
  order_amount: number;
}

export interface CouponValidationResult {
  is_valid: boolean;
  discount_amount: number;
  final_amount: number;
  coupon?: Coupon | null;
  message: string;
}

