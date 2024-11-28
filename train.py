import argparse
import numpy as np
import random
import matplotlib.pyplot as plt
import torch
from torch.utils.data import DataLoader
from torchvision.transforms import v2
import albumentations as A
import cv2

import datasets
from datasets import load_dataset
from tqdm import tqdm

from model import DeepFieldBoundaryModel
from model.color_convert import bgr2yuv, yuv2rgb
from utils import get_field_boundary_y
import multiprocess

# temporarily set this to 100Gb to speed things up
datasets.config.IN_MEMORY_MAX_SIZE = 107_374_182_400 / 2

import wandb

transform_torchvision = v2.Compose(
    [
        v2.Resize((30, 40), interpolation=v2.InterpolationMode.NEAREST),
        v2.ToDtype(torch.float32),
    ]
)

transform = A.Compose([A.Resize(30, 40, cv2.INTER_NEAREST), A.ToFloat(max_value=255)])

def function_RGBtoYUV(image):
    image_bgr = cv2.cvtColor(image, cv2.COLOR_RGB2BGR)
    image_yuv = bgr2yuv(image_bgr)
    return image_yuv

class RGBToYUV(torch.nn.Module):
    def __init__(
        self,
        y_coefficient_r=0.299,
        y_coefficient_g=0.587,
        y_coefficient_b=0.114,
        u_coefficient=0.564,
        v_coefficient=0.713,
    ):
        super().__init__()
        self.y_coefficient_r = y_coefficient_r
        self.y_coefficient_g = y_coefficient_g
        self.y_coefficient_b = y_coefficient_b
        self.u_coefficient = u_coefficient
        self.v_coefficient = v_coefficient

    def forward(self, image, targets=None, **kwargs):
        """Convert an RGB image to YCbCr.

        Args:
            image (torch.Tensor): RGB Image to be converted to YCbCr.

        Returns:
            torch.Tensor: YCbCr version of the image.
        """

        if not torch.is_tensor(image):
            raise TypeError(
                "Input type is not a torch.Tensor. Got {}".format(type(image))
            )

        if len(image.shape) < 3 or image.shape[-3] != 3:
            raise ValueError(
                "Input size must have a shape of (*, 3, H, W). Got {}".format(
                    image.shape
                )
            )

        if image.max() > 1.0 or image.min() < 0.0:
            image = image / 255.0

        r: torch.Tensor = image[..., 0, :, :]
        g: torch.Tensor = image[..., 1, :, :]
        b: torch.Tensor = image[..., 2, :, :]

        delta = 0.5
        y = (
            self.y_coefficient_r * r
            + self.y_coefficient_g * g
            + self.y_coefficient_b * b
        )
        cb = (b - y) * self.u_coefficient + delta
        cr = (r - y) * self.v_coefficient + delta

        ycbr = torch.stack((y, cb, cr), -3)

        if targets:
            return ycbr, targets
        else:
            return ycbr


def RGBtoYHS(image): #Not needed but just to have it
    image_bgr = cv2.cvtColor(image, cv2.COLOR_RGB2BGR)
    image_yuv = bgr2yuv(image_bgr)
    Y, Cr, Cb = image_yuv[..., 0], image_yuv[..., 1], image_yuv[..., 2]
    U_normed = Cb - 128
    V_normed = Cr - 128
    Hp = np.arctan2(V_normed, U_normed) * (127 / np.pi) + 127
    Sp = np.sqrt(U_normed**2 + V_normed**2) * 255.0 / Y.clip(min=1)
    Y = Y.astype(np.uint8)
    H = Hp.astype(np.uint8)
    S = Sp.astype(np.uint8)
    yhs_image = np.stack([Y, H, S], axis=-1)
    return yhs_image



rgb_to_yuv_converter = RGBToYUV()
def augmentations(batch):
    # batch["pixel_values"] = [
    #     transform(image=function_RGBtoYUV(np.array(image)))["image"].swapaxes(2, 0).swapaxes(1, 2)
    #     for image in batch["image"]
    # ]
    batch["pixel_values"] = [
        transform(
            image=rgb_to_yuv_converter(
                torch.tensor(np.array(image), dtype=torch.float32)
                .permute(2, 0, 1)
                .unsqueeze(0)
            ).squeeze(0)
            .permute(1, 2, 0)
            .numpy()
        )["image"]
        .swapaxes(2, 0)
        .swapaxes(1, 2)
        for image in batch["image"]
    ]
    batch["field_boundary"] = [
        [get_field_boundary_y(label, (0.5 + j) / 40) for j in range(40)]
        for label in batch["label"]
    ]
    return batch


def train(args):
    if args.dry_run:
        run = wandb.init(mode="offline")
    else:
        run = wandb.init(entity="nao-gait-modulation", project="spl-field-boundary", config=args) #dnt before
        # run = wandb.init(entity="dnt", project="spl-field-boundary", config=args) #dnt before

    print(args)
    # device = torch.device(args.device)
    # print(f"Device: {device}")

    # Load the dataset
    print("Loading dataset...")
    dataset = load_dataset(
        "dutchnaoteam/spl-field-boundary", cache_dir="/mnt/fishbowl/hf_cache"
    )

    print("Applying augmentations...")
    eval_dataset = load_dataset("dutchnaoteam/spl-field-boundary", split="test[:3]", cache_dir="/mnt/fishbowl/hf_cache")
    eval_dataset = eval_dataset.map(
        augmentations,
        batched=True,
        batch_size=3,
    ).with_format("torch", device=args.device)

    dataset = dataset.map(
        augmentations,
        num_proc=18,
        batched=True,
        batch_size=100,
        remove_columns=["label", "image"],
    ).with_format("torch", device=args.device)

    print("Training model...")
    train_loader = DataLoader(
        dataset["train"], batch_size=args.batch_size, drop_last=True
    )
    dev_loader = DataLoader(dataset["dev"], batch_size=args.batch_size, drop_last=True)

    device = torch.device(args.device)

    model = DeepFieldBoundaryModel(
                                    num_input=3,
                                    num_filters=args.num_filters,
                                    num_stages=args.num_stages,
                                    use_bias=args.use_bias,
                                ).to(device)
    optimizer = torch.optim.Adam(model.parameters(), lr=args.lr)
    # use l1/mean absolute error loss like in the original paper
    criterion = torch.nn.L1Loss().to(device)

    print(model)
    print(f"Number of parameters: {sum(p.numel() for p in model.parameters())}")

    pbar = tqdm(range(args.epochs), desc="Epoch", leave=False)
    run.watch(model)

    for epoch in pbar:
        train_metrics = train_for_epoch(model, train_loader, optimizer, criterion)
        dev_metrics = evaluate_model(model, dev_loader, criterion)

        examples = [
            wandb.Image(img, caption=f"example {i}")
            for i, img in enumerate(create_eval_plots(model, eval_dataset))
        ]
        run.log(
            data={
                "train_loss": train_metrics["loss"],
                "dev_loss": dev_metrics["loss"],
                "examples": examples,
            },
            commit=True,
        )

        model_file = f"trained_models/model_{epoch}.pth"
        # model.to_pt(device=device, filename=model_file)
        torch.save(model.state_dict(), model_file)

        run.save(model_file, policy="now")
        pbar.write(f"[Epoch {epoch+1}/{args.epochs}] Loss: {dev_metrics['loss']:.4f}")

    test_loader = DataLoader(dataset["test"], batch_size=args.batch_size, shuffle=True)
    test_metrics = evaluate_model(model, test_loader, criterion)
    run.log(
        {
            "test_loss": test_metrics["loss"],
        }
    )

    pbar.write(f"Test Loss: {test_metrics['loss']:.4f}")
    run.finish()


def train_for_epoch(model, data, optimizer, criterion):
    model.train()
    losses = []
    for batch in tqdm(data, desc="Train", leave=False):
        optimizer.zero_grad()
        pixel_values = batch["pixel_values"]
        # print(pixel_values.shape)
        # print(batch[0].shape)
        # pixel_values = pixel_values.transpose(1, 2, 0)
        targets = batch["field_boundary"]
        outputs = model(pixel_values)
        loss = criterion(outputs, targets)
        loss.backward()
        optimizer.step()

        losses.append(loss.item())
    return {
        "loss": np.mean(losses),
    }


def evaluate_model(model, data, criterion):
    model.eval()
    losses = []
    for batch in tqdm(data, desc="Evaluate", leave=False):
        pixel_values = batch["pixel_values"]
        targets = batch["field_boundary"]
        outputs = model(pixel_values)
        loss = criterion(outputs, targets)
        losses.append(loss.item())
    return {
        "loss": np.mean(losses),
    }


def create_eval_plots(model, data):
    model.eval()

    images = []
    for batch in data.iter(batch_size=len(data)):
        pixel_values = batch["pixel_values"]
        targets = batch["field_boundary"]
        image = batch["image"]
        outputs = model(pixel_values)
        for i in range(len(pixel_values)):
            fig = plt.figure()
            plt.imshow(image[i].cpu().numpy().transpose(1, 2, 0))
            plt.axis("off")
            plt.subplots_adjust(0, 0, 1, 1, 0, 0)
            x = np.linspace(0, image.shape[2] - 1, num=outputs.shape[1])
            plt.plot(
                x,
                np.clip(outputs[i, :].detach().cpu().numpy(), 0, 1)
                * (image.shape[1] - 1),
                color="#61fffc",
                linewidth=3,
            )
            plt.plot(
                x,
                targets[i, :].detach().cpu().numpy() * (image.shape[1] - 1),
                "r",
                linewidth=3,
            )
            fig.canvas.draw()
            images.append(
                np.frombuffer(fig.canvas.buffer_rgba(), np.uint8).reshape(480, 640, 4)
            )
            plt.close(fig)

    return images


def main():
    multiprocess.set_start_method("spawn")
    parser = argparse.ArgumentParser()
    parser.add_argument("--batch_size", type=int, default=16)
    parser.add_argument("--num_stages", type=int, default=4)
    parser.add_argument("--num_filters", type=int, default=16)
    parser.add_argument("--use_bias", action="store_true", default=False)
    parser.add_argument("--lr", type=float, default=0.001)
    parser.add_argument("--epochs", type=int, default=20)
    parser.add_argument(
        "--device", type=str, default="cuda" if torch.cuda.is_available() else "cpu"
    )
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--dry-run", action="store_true", default=False)
    args = parser.parse_args()

    print(f"Seed: {args.seed}")

    torch.manual_seed(args.seed)
    torch.backends.cudnn.deterministic = True
    torch.cuda.manual_seed_all(args.seed)
    np.random.seed(args.seed)
    random.seed(args.seed)

    train(args)


if __name__ == "__main__":
    main()
