import Stripe from "stripe";

let stripe: Stripe | null = null;
function getStripe(): Stripe {
  if (!stripe) {
    if (!process.env.STRIPE_API_KEY) {
      throw new Error("STRIPE_API_KEY is not set");
    }
    stripe = new Stripe(process.env.STRIPE_API_KEY);
  }
  return stripe;
}

export async function checkCustomerPaymentMethodSet(stripeCustomerId: string) {
  const resp = await getStripe().customers.listPaymentMethods(stripeCustomerId);
  if (resp.data && resp.data.length > 0) {
    return true;
  }

  return false;
}
