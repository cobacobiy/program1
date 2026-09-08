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
  weight_grams?: number;
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
  address_id?: string;
  courier?: string;
  shipping_cost_cents?: number;
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
  status: string;
  tracking_number?: string | null;
  courier?: string | null;
  shipping_cost_cents?: number;
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

export interface ProductReview {
  id: string;
  product_id: string;
  buyer_id: string;
  buyer_name: string;
  order_id: string;
  rating: number;
  review_text?: string | null;
  is_visible: boolean;
  created_at: string;
  updated_at: string;
}

export interface ProductRatingSummary {
  product_id: string;
  average_rating: number;
  total_reviews: number;
  rating_distribution: [number, number, number, number, number];
}

export interface CreateReviewPayload {
  product_id: string;
  order_id: string;
  rating: number;
  review_text?: string | null;
}

export interface PaginatedReviews {
  data: ProductReview[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export interface PublicReview {
  id: string;
  product_id: string;
  buyer_name: string;
  rating: number;
  review_text?: string | null;
  created_at: string;
  updated_at: string;
}

export interface PaginatedPublicReviews {
  data: PublicReview[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export interface WishlistItem {
  product_id: string;
  product_name: string;
  product_sku: string;
  product_price_cents: number;
  product_image_url: string;
  stock: number;
  added_at: string;
}

export interface BuyerAddress {
  id: string;
  recipient_name: string;
  phone_number: string;
  street_address: string;
  subdistrict: string;
  city: string;
  province: string;
  postal_code: string;
  is_default: boolean;
}

export interface CourierRate {
  courier_code: string;
  courier_name: string;
  service: string;
  service_description: string;
  cost_cents: number;
  etd: string;
}

export interface ShippingCostRequest {
  destination_city_id: string;
  weight_grams: number;
  courier: string;
}

export interface ShippingCity {
  city_id: string;
  province_id: string;
  province: string;
  city_name: string;
  postal_code: string;
}

export interface SalesReportQuery {
  date_from?: string;
  date_to?: string;
  status_filter?: string;
}

export interface SalesReportItem {
  product_name: string;
  quantity: number;
  unit_price_cents: number;
}

export interface SalesReportRow {
  order_id: string;
  order_date: string;
  buyer_name: string;
  buyer_email?: string | null;
  items: SalesReportItem[];
  subtotal_cents: number;
  shipping_cents: number;
  discount_cents: number;
  total_cents: number;
  status: string;
  payment_status: string;
}

export interface SalesReportSummary {
  total_revenue_cents: number;
  total_orders: number;
  average_order_value_cents: number;
  total_items_sold: number;
  orders_by_status: Record<string, number>;
}

export interface SalesReportResponse {
  summary: SalesReportSummary;
  rows: SalesReportRow[];
}
